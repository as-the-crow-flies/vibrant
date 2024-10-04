pub mod camera;
pub mod constants;
pub mod tractogram_baseline;
pub mod tractogram_compute;
pub mod ui;
pub mod uv;

use camera::Camera;
use constants::Constants;
use tractogram_baseline::BaselineTractogramRenderer;
use tractogram_compute::ComputeTractogramRenderer;
use ui::UiRenderer;
use uv::UvRenderer;
use wgpu::SurfaceTarget;

use crate::{
    asset::{Asset, Tractogram},
    loader::AssetLoader,
};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    gpu: Gpu,
    surface: Option<Surface>,
    constants: Constants,

    asset: Asset,

    camera: Camera,
    uv: UvRenderer,
    tractogram_baseline: BaselineTractogramRenderer,
    // tractogram_compute: ComputeTractogramRenderer,
    ui: UiRenderer,
}

impl Renderer {
    pub fn new(gpu: Gpu) -> Self {
        let constants = Constants::new(&gpu, (1, 1), 512);

        Self {
            surface: None,

            uv: UvRenderer::new(&gpu),
            tractogram_baseline: BaselineTractogramRenderer::new(&gpu),
            // tractogram_compute: ComputeTractogramRenderer::new(&gpu, &constants),
            ui: UiRenderer::new(&gpu),

            camera: Camera::new(&gpu),
            asset: Asset::default(),

            constants,
            gpu,
        }
    }

    pub fn create_surface(&mut self, window: impl Into<SurfaceTarget<'static>>) {
        self.surface = Some(Surface::new(&self.gpu, window))
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface
            .as_mut()
            .unwrap()
            .resize(&self.gpu, width, height);

        self.constants = Constants::new(&self.gpu, (width, height), 256);
        // self.tractogram_compute = ComputeTractogramRenderer::new(&self.gpu, &self.constants);
    }

    pub fn update(&mut self, controller: &Controller) {
        AssetLoader::on_tractogram(|tractogram| {
            self.asset.tractogram = Some(Tractogram::new(&self.gpu, &tractogram))
        });

        self.camera.update(&self.gpu, controller.camera().mvp());
    }

    pub fn render(&mut self, ctx: &egui::Context, output: egui::FullOutput) {
        let surface_frame = self
            .surface
            .as_ref()
            .expect("Surface was not initialized")
            .surface_frame(&self.gpu);

        let frame = surface_frame.frame();

        let mut cmd = self.gpu.cmd();

        self.uv.render(&mut cmd, &frame);

        if let Some(tractogram) = &self.asset.tractogram {
            self.tractogram_baseline
                .render(&mut cmd, &self.camera, &frame, tractogram);

            // self.tractogram_compute
            //     .render(&mut cmd, &self.camera, &frame, tractogram);
        }

        self.ui.render(&self.gpu, &mut cmd, &frame, ctx, output);

        self.gpu.submit(cmd);

        surface_frame.present();

        self.gpu.wait();
    }
}
