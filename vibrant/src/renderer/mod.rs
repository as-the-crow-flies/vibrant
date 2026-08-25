pub mod anatomy;
pub mod environment;
pub mod line;
pub mod ui;
pub mod util;
pub mod wgsl;

use std::sync::Arc;

use crate::renderer::{anatomy::AnatomyRenderer, line::LineRenderer, util::clear::ClearPipeline};
use environment::Environment;
use pollster::FutureExt;
use ui::UiRenderer;
use winit::window::Window;

use crate::{asset::Asset, file::FileStage};

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
        Self {
            egui: egui_winit::State::new(
                egui::Context::default(),
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
        self.surface.maybe_resize(gpu, controller.settings_mut());

        let input = self.egui.take_egui_input(window);
        let mut output = self.egui.egui_ctx().run_ui(input, |ui| {
            controller.ui(ui, &mut self.asset, window.scale_factor() as f32, dt)
        });
        self.egui
            .handle_platform_output(&window, output.platform_output.clone());

        self.environment.update(gpu, &controller);
        self.asset.update(gpu, &controller);

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

            if !FileStage::about_to_save() {
                let ctx = self.egui.egui_ctx();
                self.ui.paint(
                    gpu,
                    &mut cmd,
                    frame,
                    ctx,
                    output.shapes,
                    output.pixels_per_point,
                );
            }

            self.surface.present(gpu, cmd);

            FileStage::on_save(|path| gpu.save(path, frame.color().texture()).block_on());
        }

        output.textures_delta.clear();
    }
}
