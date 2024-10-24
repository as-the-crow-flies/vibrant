pub mod constants;
pub mod environment;
pub mod services;
pub mod tractogram;
pub mod ui;
pub mod volume;

use constants::Constants;
use environment::Environment;
use tractogram::density::compute::TractogramDensityComputeRenderer;
use tractogram::full::render::TractogramRenderer;
use tractogram::line::compute::TractogramLineComputeRenderer;
use tractogram::line::render::TractogramLineRenderRenderer;
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
    tractogram_density: TractogramDensityComputeRenderer,
    tractogram_baseline: TractogramLineRenderRenderer,
    tractogram_render: TractogramRenderer,
    tractogram_compute_renderer: TractogramLineComputeRenderer,
    volume_render: VolumeRenderer,
    ui: UiRenderer,
}

impl Renderer {
    const VOLUME_EXPONENT: u32 = 8;

    pub fn new(gpu: Gpu) -> Self {
        let asset = Asset {
            tractogram: None,
            volume: None,
            density: Density::new(&gpu, Self::VOLUME_EXPONENT),
        };

        let constants = Constants::new(&gpu, (1, 1), asset.density.size());

        Self {
            surface: None,

            tractogram_density: TractogramDensityComputeRenderer::new(&gpu, &constants),
            tractogram_baseline: TractogramLineRenderRenderer::new(&gpu),
            tractogram_render: TractogramRenderer::new(&gpu, &constants),
            tractogram_compute_renderer: TractogramLineComputeRenderer::new(&gpu, &constants),
            volume_render: VolumeRenderer::new(&gpu),
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

        self.tractogram_density = TractogramDensityComputeRenderer::new(&self.gpu, &self.constants);
        self.tractogram_render = TractogramRenderer::new(&self.gpu, &self.constants);
        self.tractogram_compute_renderer =
            TractogramLineComputeRenderer::new(&self.gpu, &self.constants);
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
            .surface_frame(&self.gpu);

        let frame = surface_frame.frame();

        let mut cmd = self.gpu.cmd();

        if let Some(tractogram) = &self.asset.tractogram {
            match controller.settings().renderer {
                crate::controller::settings::Renderer::Baseline => {
                    self.tractogram_baseline
                        .render(&mut cmd, &self.environment, frame, tractogram);
                }
                crate::controller::settings::Renderer::Compute => {
                    self.tractogram_compute_renderer.render(
                        &mut cmd,
                        &self.environment,
                        &frame,
                        tractogram,
                    );
                }
                crate::controller::settings::Renderer::Regular => {
                    self.tractogram_density.render(
                        &mut cmd,
                        &self.environment,
                        tractogram,
                        &self.asset.density,
                    );
                    self.tractogram_render.render(
                        &mut cmd,
                        &self.environment,
                        &frame,
                        tractogram,
                        &self.asset.density,
                    );
                }
            };
        }

        if let Some(volume) = &self.asset.volume {
            self.volume_render
                .render(&mut cmd, &self.environment, frame, volume);
        }

        self.ui.render(&self.gpu, &mut cmd, &frame, ctx, output);

        self.gpu.submit(cmd);

        surface_frame.present();

        self.gpu.wait();
    }
}
