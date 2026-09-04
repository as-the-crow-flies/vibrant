pub mod accumulate;
pub mod gradient;
pub mod lighting;
pub mod line;
pub mod present;
pub mod ui;
pub mod util;
pub mod volume;
pub mod wgsl;

use std::sync::Arc;
use std::time::Duration;

use crate::renderer::{
    gradient::GradientPipeline, lighting::LightingRenderer, line::LineRenderer,
    util::clear::ClearPipeline, volume::VolumeRenderer,
};
use ui::UiRenderer;
use wgpu::RenderPassDescriptor;
use winit::window::Window;

use crate::{asset::Asset, file::FileStage, gpu::readback::spawn_task};

use super::{controller::Controller, gpu::Gpu, surface::Surface};
use crate::controller::settings::RenderMode;

/// What [`Renderer::render`] wants the event loop to do next.
pub struct RenderOutcome {
    /// The image is still progressively refining - schedule another frame as
    /// soon as the GPU goes idle.
    pub accumulating: bool,
    /// A file load is in progress - keep the loop rendering so the spinner
    /// animates and the built scene appears without needing further input.
    pub loading: bool,
    /// egui's requested delay until the next repaint (`Duration::MAX` = none).
    pub repaint_after: Duration,
}

pub struct Renderer {
    surface: Surface,

    clear: ClearPipeline,
    volume: VolumeRenderer,
    gradient: GradientPipeline,
    lighting: LightingRenderer,
    line: LineRenderer,
    ui: UiRenderer,

    asset: Asset,
}

impl Renderer {
    pub fn new(gpu: &Gpu, window: Arc<Window>) -> Self {
        Self {
            ui: UiRenderer::new(gpu, &window),
            surface: Surface::new(gpu, window),

            clear: ClearPipeline::new(gpu),

            volume: VolumeRenderer::new(gpu),
            gradient: GradientPipeline::new(gpu),
            lighting: LightingRenderer::new(gpu),
            line: LineRenderer::new(gpu),

            asset: Asset::new(gpu),
        }
    }

    pub fn ui(&mut self) -> &mut UiRenderer {
        &mut self.ui
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        window: &Arc<Window>,
        controller: &mut Controller,
        dt: f32,
    ) -> RenderOutcome {
        self.surface.maybe_reconfigure(gpu, controller.settings());

        let repaint_after = self.ui.run(gpu, window, |ui| {
            controller.ui(
                ui,
                &mut self.asset,
                window.scale_factor() as f32,
                dt,
                self.surface
                    .hdr_supported()
                    .then(|| self.surface.hdr_headroom_limit(gpu)),
                self.surface.accumulation(),
            )
        });

        self.asset.update(gpu, controller);

        // Anything the surface can't see for itself that invalidates the
        // accumulated image. Camera movement is polled here (not covered by
        // `lighting_changed`); the view-independent radiance cascades keep their
        // own gate, so a camera-only move just re-traces per sample.
        let scene_dirty =
            self.asset.changed() | controller.lighting_changed() | controller.take_camera_changed();

        let settings = *controller.settings();
        let plan = self.surface.plan_accumulation(&settings, scene_dirty);

        self.asset.environment.update(gpu, controller, plan.jitter);

        if let Some(frame) = self.surface.frame() {
            let mut cmd = gpu.cmd();

            if plan.render {
                self.clear.dispatch(&mut cmd, frame.color());

                // Clear the combined-mode line-depth target to "far". The opaque
                // line pass overwrites hit pixels; the volume tracer's `min`
                // clamp is a no-op wherever it stays 1.0 (no line, or lines off).
                cmd.begin_render_pass(&RenderPassDescriptor {
                    label: Some("LineDepthClear"),
                    color_attachments: &[Some(frame.line_depth().attachment_clear_far())],
                    ..Default::default()
                });

                let combined = settings.render_mode == RenderMode::Combined;

                let data_changed = self.asset.changed() || controller.changed();
                let lighting_changed = data_changed || controller.light().changed();

                if data_changed {
                    // Occupancy pyramid first: the line deposit samples it.
                    self.line.transfer(&mut cmd, controller, &self.asset);

                    if let Some(pv) = &self.asset.physical_volume {
                        // Volume fractions -> the marched textures (line-free).
                        self.volume.transfer(&mut cmd, &self.asset, pv);
                        self.gradient.dispatch(&mut cmd, pv);
                        // Neutral line density -> `pv.line_extinction` (a
                        // separate texture the volume tracer never reads).
                        self.line.deposit(&mut cmd, controller, &self.asset);
                    }
                }

                if lighting_changed {
                    if let Some(pv) = &self.asset.physical_volume {
                        // Primary cascade. Combined: volume + lines. X-ray:
                        // volume only. Sources are baked into each radiance
                        // buffer's `cascade_opts` by `Asset::sync_physical_volume`.
                        if let Some(rad) = &self.asset.radiance {
                            self.lighting
                                .dispatch_for(&mut cmd, &self.asset, controller, rad, pv);
                        }
                        // X-ray only: the line-only cascade.
                        if let Some(rad) = &self.asset.radiance_lines {
                            self.lighting
                                .dispatch_for(&mut cmd, &self.asset, controller, rad, pv);
                        }
                    }
                }

                if combined {
                    // Lines first (opaque, writes depth `s`), then the volume
                    // clamped to it and composited over.
                    self.line.render(&mut cmd, controller, &self.asset, frame);
                    self.volume
                        .render(gpu, &mut cmd, controller, &self.asset, frame);
                } else {
                    // Volume first, then transparent lines composited on top.
                    self.volume
                        .render(gpu, &mut cmd, controller, &self.asset, frame);
                    self.line.render(&mut cmd, controller, &self.asset, frame);
                }
            }

            // Fold the fresh sample into the running mean (when accumulating)
            // and hand back the buffer the present/export passes tone map.
            let scene = self
                .surface
                .resolve(gpu, &mut cmd, &settings, plan.render)
                .expect("frame present");

            self.ui.paint(gpu, &mut cmd, frame);

            // A pending screenshot needs the SDR-composited image. Record the
            // export pass into `cmd`, but only kick off the (blocking, on
            // native) readback *after* `present` has submitted `cmd` - otherwise
            // `gpu.save` copies the export texture before it has been written.
            let mut save_path = None;
            FileStage::on_save(|path| save_path = Some(path));

            if save_path.is_some() {
                self.surface.export(gpu, &mut cmd, frame, scene);
            }

            self.surface.present(gpu, cmd, scene);

            if let Some(path) = save_path {
                let gpu = gpu.clone();
                let texture = frame.export().texture().clone();
                let viewport = controller.viewport();
                spawn_task(async move { gpu.save(path, &texture, viewport).await });
            }
        }

        // Fold any finished convergence reading in and, on the interval, kick off
        // the next readback (the `reduce` pass was just submitted by `present`).
        self.surface.read_metric(gpu, &settings);

        RenderOutcome {
            accumulating: self.surface.accumulating(&settings),
            loading: FileStage::loading(),
            repaint_after,
        }
    }
}
