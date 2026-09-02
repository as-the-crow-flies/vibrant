pub mod anatomy;
pub mod environment;
pub mod line;
pub mod present;
pub mod ui;
pub mod util;
pub mod wgsl;

use std::sync::Arc;

use crate::renderer::{anatomy::AnatomyRenderer, line::LineRenderer, util::clear::ClearPipeline};
use environment::Environment;
use ui::UiRenderer;
use winit::window::Window;

use crate::{asset::Asset, file::FileStage, gpu::readback::spawn_task};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    surface: Surface,
    egui: egui_winit::State,

    clear: ClearPipeline,
    anatomy: AnatomyRenderer,
    line: LineRenderer,
    ui: UiRenderer,

    environment: Environment,
    asset: Asset,
}

impl Renderer {
    pub fn new(gpu: &Gpu, window: Arc<Window>) -> Self {
        let egui_ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        crate::controller::icons::install(&mut fonts);
        egui_ctx.set_fonts(fonts);

        Self {
            egui: egui_winit::State::new(
                egui_ctx,
                egui::viewport::ViewportId::ROOT,
                &window,
                Some(window.scale_factor() as f32),
                None,
                None,
            ),
            surface: Surface::new(gpu, window),

            clear: ClearPipeline::new(gpu),

            anatomy: AnatomyRenderer::new(gpu),
            line: LineRenderer::new(gpu),
            ui: UiRenderer::new(gpu),

            environment: Environment::new(gpu),
            asset: Asset::new(gpu),
        }
    }

    pub fn egui(&mut self) -> &mut egui_winit::State {
        &mut self.egui
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        window: &Arc<Window>,
        controller: &mut Controller,
        dt: f32,
    ) {
        self.surface.maybe_reconfigure(gpu, controller.settings());

        // `Some(limit)` = HDR available, and the max peak the display can drive
        // right now; `None` = HDR unavailable.
        let hdr_headroom_limit = self
            .surface
            .hdr_supported()
            .then(|| self.surface.hdr_headroom_limit(gpu));

        let input = self.egui.take_egui_input(window);
        let mut output = self.egui.egui_ctx().run_ui(input, |ui| {
            controller.ui(
                ui,
                &mut self.asset,
                window.scale_factor() as f32,
                dt,
                hdr_headroom_limit,
            )
        });
        self.egui
            .handle_platform_output(&window, output.platform_output.clone());

        self.environment.update(gpu, &controller);
        self.asset.update(gpu, controller);

        // Texture updates/frees must be applied regardless of whether we end up painting
        // this frame, otherwise egui's `TexturesDelta` is dropped unhandled.
        self.ui.update_textures(gpu, &output.textures_delta);

        if let Some(frame) = self.surface.frame() {
            let mut cmd = gpu.cmd();

            self.clear.dispatch(&mut cmd, frame.color());

            self.anatomy.render(
                &mut cmd,
                controller,
                &self.environment,
                &self.surface,
                &self.asset,
            );

            self.line.render(
                &mut cmd,
                controller,
                &self.environment,
                &self.surface,
                &self.asset,
            );

            self.ui.paint(
                gpu,
                &mut cmd,
                frame,
                self.egui.egui_ctx(),
                output.shapes,
                output.pixels_per_point,
            );

            // A pending screenshot needs the SDR-composited image. Record the
            // export pass into `cmd`, but only kick off the (blocking, on
            // native) readback *after* `present` has submitted `cmd` - otherwise
            // `gpu.save` copies the export texture before it has been written.
            let mut save_path = None;
            FileStage::on_save(|path| save_path = Some(path));

            if save_path.is_some() {
                self.surface.export(gpu, &mut cmd, frame);
            }

            self.surface.present(gpu, cmd);

            if let Some(path) = save_path {
                let gpu = gpu.clone();
                let texture = frame.export().texture().clone();
                let viewport = controller.viewport();
                spawn_task(async move { gpu.save(path, &texture, viewport).await });
            }
        }

        output.textures_delta.clear();
    }
}
