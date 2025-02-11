pub mod environment;
pub mod services;
pub mod tractogram;
pub mod ui;

use crate::{file::Tck, renderer::tractogram::TractogramRenderer};
use environment::Environment;
use pollster::FutureExt;
use ui::UiRenderer;
use wgpu::SurfaceTarget;

use crate::{
    asset::{tractogram::Tractogram, Asset},
    file::File,
};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    surface: Surface,
    tractogram: TractogramRenderer,
    ui: UiRenderer,
    environment: Environment,
    asset: Asset,
}

impl Renderer {
    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        Self {
            surface: Surface::new(gpu, window),
            tractogram: TractogramRenderer::new(gpu),
            ui: UiRenderer::new(gpu),

            environment: Environment::new(gpu),
            asset: Asset { tractogram: None },
        }
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        controller: &Controller,
        ctx: &egui::Context,
        output: egui::FullOutput,
    ) {
        File::on_tck(|tck| self.asset.tractogram = Some(Tractogram::new(gpu, &tck)));

        // if self.asset.tractogram.is_none() {
        //     self.asset.tractogram = Some(Tractogram::new(
        //         gpu,
        //         &Tck::from_file("assets/HCP-100307/whole_brain200k.tck"),
        //     ));
        // }

        let surface = self.surface.maybe_resize(
            gpu,
            controller.width(),
            controller.height(),
            controller.volume(),
            controller.tile(),
        );

        self.environment.update(gpu, &controller);

        let mut cmd = gpu.cmd();

        if let Some(tractogram) = &self.asset.tractogram {
            self.tractogram.render(
                &mut cmd,
                &self.environment,
                surface.buffer(),
                tractogram,
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
