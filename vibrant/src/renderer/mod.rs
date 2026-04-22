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
            asset: Asset::default(),
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
        self.asset.update(gpu);

        let surface = self.surface.maybe_resize(gpu, &controller.settings());

        let input = self.egui.take_egui_input(window);
        let output = self
            .egui
            .egui_ctx()
            .run_ui(input, |ui| controller.ui(ui, &mut self.asset, dt));
        self.egui
            .handle_platform_output(&window, output.platform_output.clone());

        self.environment.update(gpu, &controller);

        if let Some(line) = &self.asset.line {
            line.update_settings(gpu);
        }
        if let Some(hdri) = &self.asset.hdri {
            hdri.update_settings(gpu);
        }
        if let Some(mask) = &self.asset.mask {
            mask.update_settings(gpu);
        }
        for volume in &self.asset.volumes {
            volume.update_settings(gpu);
        }

        let mut cmd = gpu.cmd();

        self.clear.dispatch(&mut cmd, surface.frame().post());

        self.anatomy.render(
            &mut cmd,
            controller,
            &self.environment,
            surface.frame(),
            &self.asset,
        );

        if let Some(line) = &self.asset.line {
            self.line.render(
                &mut cmd,
                controller,
                &self.environment,
                surface.frame(),
                line,
            );
        };

        if !FileStage::about_to_save() {
            self.ui
                .render(gpu, &mut cmd, surface.frame(), self.egui.egui_ctx(), output);
        }

        surface.present(gpu, cmd);

        FileStage::on_save(|path| gpu.save(path, surface.frame().color().texture()).block_on());
    }
}
