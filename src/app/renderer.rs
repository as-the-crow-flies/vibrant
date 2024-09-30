pub mod camera;
pub mod tractogram;
pub mod uv;

use std::sync::Arc;

use camera::Camera;
use rfd::AsyncFileDialog;
use tractogram::TractogramRenderer;
use uv::UvRenderer;
use wgpu::CommandEncoderDescriptor;
use winit::{dpi::PhysicalSize, window::Window};

use super::{controller::Controller, gpu::Gpu, loader::Tractogram, surface::Surface};

pub struct Renderer {
    gpu: Gpu,
    surface: Option<Surface>,

    camera: Camera,
    uv: UvRenderer,
    tractogram: TractogramRenderer,
}

impl Renderer {
    pub async fn new() -> Self {
        let gpu = Gpu::new().await;
        let camera = Camera::new(&gpu);

        let tractogram = Tractogram::from_file_dialog()
            .await
            .expect("Please choose a Tractogram file");

        Self {
            surface: None,
            uv: UvRenderer::new(&gpu),
            tractogram: TractogramRenderer::new(&gpu, &camera, &tractogram),

            camera,
            gpu,
        }
    }

    pub fn create_surface(&mut self, window: Arc<Window>) {
        self.surface = Some(Surface::new(&self.gpu, window))
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface
            .as_mut()
            .expect("Surface is not initialized")
            .resize(&self.gpu, size.width, size.height);
    }

    pub fn render(&self, controller: &Controller) {
        self.camera.update(&self.gpu, controller.camera());

        let frame = self
            .surface
            .as_ref()
            .expect("Surface is not initialized")
            .frame();

        let view = frame.view();

        let mut cmd = self
            .gpu
            .device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        self.uv.render(&mut cmd, &view);
        self.tractogram.render(&mut cmd, &self.camera, &view);

        self.gpu.queue().submit([cmd.finish()]);

        frame.present();
    }
}
