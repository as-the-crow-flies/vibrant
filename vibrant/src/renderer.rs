pub mod constants;
pub mod debug;
pub mod environment;
pub mod services;
pub mod tractogram;
pub mod ui;
pub mod volume;

use crate::{asset::occlusion::Occlusion, renderer::tractogram::TractogramRenderer};
use constants::Constants;
use debug::DebugRenderer;
use environment::Environment;
use ui::UiRenderer;
use volume::compute::VolumeRenderer;
use wgpu::SurfaceTarget;

use crate::{
    asset::{density::Density, Asset, Tractogram, Volume},
    loader::AssetLoader,
};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    gpu: Gpu,
    surface: Option<Surface>,
    constants: Constants,

    asset: Asset,

    environment: Environment,
    tractogram: TractogramRenderer,
    volume_render: VolumeRenderer,

    debug: DebugRenderer,

    ui: UiRenderer,
}

impl Renderer {
    const VOLUME_EXPONENT: u32 = 7;

    pub fn new(gpu: Gpu) -> Self {
        let asset = Asset {
            tractogram: None,
            volume: None,
            density: Density::new(&gpu, Self::VOLUME_EXPONENT),
            occlusion: Occlusion::new(&gpu, Self::VOLUME_EXPONENT),
        };

        let constants = Constants::new(&gpu, (1, 1), asset.density.size());

        Self {
            surface: None,

            tractogram: TractogramRenderer::new(&gpu, &constants),
            volume_render: VolumeRenderer::new(&gpu),
            debug: DebugRenderer::new(&gpu),
            ui: UiRenderer::new(&gpu),

            environment: Environment::new(&gpu),
            asset,

            constants,
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

        self.constants = Constants::new(&self.gpu, (width, height), self.asset.density.size());
        self.tractogram = TractogramRenderer::new(&self.gpu, &self.constants)
    }

    pub fn render(
        &mut self,
        controller: &Controller,
        ctx: &egui::Context,
        output: egui::FullOutput,
    ) {
        AssetLoader::on_tck(|tck| self.asset.tractogram = Some(Tractogram::new(&self.gpu, &tck)));
        AssetLoader::on_nifti(|nifti| self.asset.volume = Some(Volume::new(&self.gpu, &nifti)));

        self.environment.update(&self.gpu, &controller);

        let surface_frame = self
            .surface
            .as_ref()
            .expect("Surface was not initialized")
            .surface_frame();

        let frame = surface_frame.frame();

        let mut cmd = self.gpu.cmd();

        if let Some(tractogram) = &self.asset.tractogram {
            self.tractogram.render(
                &mut cmd,
                &self.environment,
                frame,
                tractogram,
                &self.asset.density,
                &self.asset.occlusion,
                &controller.settings().geometry,
            );
        }

        if let Some(volume) = &self.asset.volume {
            self.volume_render
                .render(&mut cmd, &self.environment, frame, volume);
        }

        self.debug
            .render(&mut cmd, frame, &self.environment, controller.settings());

        self.ui.render(&self.gpu, &mut cmd, &frame, ctx, output);

        self.gpu.submit(cmd);

        surface_frame.present();
    }
}
