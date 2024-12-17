pub mod environment;
pub mod services;
pub mod tractogram;
pub mod ui;

use crate::{asset::density::Density, renderer::tractogram::TractogramRenderer};
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
    gpu: Gpu,
    surface: Option<Surface>,

    asset: Asset,

    environment: Environment,
    tractogram: TractogramRenderer,

    ui: UiRenderer,
}

impl Renderer {
    pub fn new(gpu: Gpu) -> Self {
        let asset = Asset {
            tractogram: None,
            density: Density::new(&gpu),
        };

        Self {
            surface: None,

            tractogram: TractogramRenderer::new(&gpu),
            ui: UiRenderer::new(&gpu),

            environment: Environment::new(&gpu),
            asset,

            gpu,
        }
    }

    pub fn create_surface(&mut self, window: impl Into<SurfaceTarget<'static>>) {
        self.surface = Some(Surface::new(&self.gpu, window));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface
            .as_mut()
            .unwrap()
            .resize(&self.gpu, width, height);
    }

    pub fn render(
        &mut self,
        controller: &Controller,
        ctx: &egui::Context,
        output: egui::FullOutput,
    ) {
        File::on_tck(|tck| self.asset.tractogram = Some(Tractogram::new(&self.gpu, &tck)));

        self.environment.update(&self.gpu, &controller);

        let surface = self.surface.as_ref().expect("Surface was not initialized");

        let mut cmd = self.gpu.cmd();

        if let Some(tractogram) = &self.asset.tractogram {
            self.tractogram.render(
                &mut cmd,
                &self.environment,
                surface.buffer(),
                &self.asset.density,
                tractogram,
                controller.settings(),
            );
        }

        if !File::about_to_save() {
            self.ui
                .render(&self.gpu, &mut cmd, surface.buffer(), ctx, output);
        }

        surface.present(&self.gpu, cmd);

        File::on_save(|path| {
            self.gpu
                .save(path, surface.buffer().color().texture())
                .block_on()
        });
    }
}
