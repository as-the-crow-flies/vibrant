pub mod constants;
pub mod environment;
pub mod tractogram_baseline;
pub mod tractogram_density;
pub mod tractogram_render;
pub mod ui;
pub mod uv;

use constants::Constants;
use environment::Environment;
use tractogram_density::TractogramDensityRenderer;
use tractogram_render::TractogramRenderer;
use ui::UiRenderer;
use wgpu::SurfaceTarget;

use crate::{
    asset::{density::Density, Asset, Tractogram},
    loader::{self, AssetLoader},
};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    gpu: Gpu,
    surface: Option<Surface>,
    constants: Constants,

    asset: Asset,

    camera: Environment,
    tractogram_density: TractogramDensityRenderer,
    tractogram_render: TractogramRenderer,
    ui: UiRenderer,
}

impl Renderer {
    const VOLUME_EXPONENT: u32 = 8;

    pub fn new(gpu: Gpu) -> Self {
        let asset = Asset {
            tractogram: None,
            density: Density::new(&gpu, Self::VOLUME_EXPONENT),
        };

        let constants = Constants::new(&gpu, (1, 1), asset.density.size());

        Self {
            surface: None,

            tractogram_density: TractogramDensityRenderer::new(&gpu, &constants),
            tractogram_render: TractogramRenderer::new(&gpu, &constants),
            ui: UiRenderer::new(&gpu),

            camera: Environment::new(&gpu),
            asset,

            constants,
            gpu,
        }
    }

    pub fn create_surface(&mut self, window: impl Into<SurfaceTarget<'static>>) {
        self.surface = Some(Surface::new(&self.gpu, window));

        // AssetLoader::publish_tractogram(loader::Tractogram::from_file(
        //     "assets/whole_brain200k.tck",
        // ));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface
            .as_mut()
            .unwrap()
            .resize(&self.gpu, width, height);

        self.constants = Constants::new(&self.gpu, (width, height), self.asset.density.size());

        self.tractogram_density = TractogramDensityRenderer::new(&self.gpu, &self.constants);
        self.tractogram_render = TractogramRenderer::new(&self.gpu, &self.constants);
    }

    pub fn update(&mut self, controller: &Controller) {
        AssetLoader::on_tractogram(|tractogram| {
            self.asset.tractogram = Some(Tractogram::new(&self.gpu, &tractogram));
        });

        self.camera.update(&self.gpu, &controller);
    }

    pub fn render(&mut self, ctx: &egui::Context, output: egui::FullOutput) {
        let surface_frame = self
            .surface
            .as_ref()
            .expect("Surface was not initialized")
            .surface_frame(&self.gpu);

        let frame = surface_frame.frame();

        let mut cmd = self.gpu.cmd();

        if let Some(tractogram) = &self.asset.tractogram {
            self.tractogram_density
                .render(&mut cmd, &self.camera, tractogram, &self.asset.density);

            self.tractogram_render.render(
                &mut cmd,
                &self.camera,
                &frame,
                tractogram,
                &self.asset.density,
            );
        }

        self.ui.render(&self.gpu, &mut cmd, &frame, ctx, output);

        self.gpu.submit(cmd);

        surface_frame.present();

        self.gpu.wait();
    }
}
