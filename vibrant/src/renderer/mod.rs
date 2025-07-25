pub mod environment;
pub mod line;
pub mod ui;
pub mod wgsl;

use crate::renderer::line::LineRenderer;
use environment::Environment;
use pollster::FutureExt;
use ui::UiRenderer;
use wgpu::SurfaceTarget;

use crate::{
    asset::{line::LineSet, Asset},
    file::File,
};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    surface: Surface,
    line: LineRenderer,
    ui: UiRenderer,
    environment: Environment,
    asset: Asset,
}

impl Renderer {
    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        Self {
            surface: Surface::new(gpu, window),
            line: LineRenderer::new(gpu),
            ui: UiRenderer::new(gpu),

            environment: Environment::new(gpu),
            asset: Asset { line: None },
        }
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        controller: &Controller,
        ctx: &egui::Context,
        output: egui::FullOutput,
    ) {
        File::on_line(|tck| {
            self.asset.line = Some(LineSet::new(gpu, &tck));
        });

        let surface = self.surface.maybe_resize(gpu, &controller.settings());

        self.environment.update(gpu, &controller);

        let mut cmd = gpu.cmd();

        if let Some(line) = &self.asset.line {
            self.line.render(
                &mut cmd,
                &self.environment,
                surface.buffer(),
                line,
                controller.settings(),
            );
        }

        if !File::about_to_save() {
            self.ui.render(gpu, &mut cmd, surface.buffer(), ctx, output);
        }

        surface.present(gpu, cmd);

        File::on_save(|path| {
            gpu.save(path, surface.buffer().color().texture())
                .block_on()
        });
    }
}
