pub mod camera;
pub mod tractogram;
pub mod ui;
pub mod uv;

use camera::Camera;
use tractogram::TractogramRenderer;
use ui::UiRenderer;
use uv::UvRenderer;
use wgpu::SurfaceTarget;

use crate::{buffer::AssetBuffer, loader::AssetLoader};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    gpu: Gpu,
    surface: Option<Surface>,
    assets: AssetBuffer,

    camera: Camera,
    uv: UvRenderer,
    tractogram: TractogramRenderer,
    ui: UiRenderer,
}

impl Renderer {
    pub fn new(gpu: Gpu) -> Self {
        let assets = AssetBuffer::new(&gpu);
        let camera = Camera::new(&gpu);

        Self {
            surface: None,

            uv: UvRenderer::new(&gpu),
            tractogram: TractogramRenderer::new(&gpu, &camera, assets.tractogram()),
            ui: UiRenderer::new(&gpu),

            camera,
            assets,
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
    }

    pub fn update(&mut self, controller: &Controller) {
        AssetLoader::on_tractogram(|tractogram| self.assets.set_tractogram(&self.gpu, tractogram));

        self.camera.update(&self.gpu, controller.camera().mvp());
    }

    pub fn render(&mut self, ctx: &egui::Context, output: egui::FullOutput) {
        let frame = self.surface.as_ref().unwrap().get_current_frame();

        let view = frame.create_view();

        let mut cmd = self.gpu.cmd();

        self.uv.render(&mut cmd, &view);
        self.tractogram.render(&mut cmd, &self.camera, &view);
        self.ui.render(&self.gpu, &mut cmd, &view, ctx, output);

        self.gpu.submit(cmd);

        frame.present();

        self.gpu.wait();
    }
}
