pub mod tractogram;
pub mod uv;

use tractogram::TractogramRenderer;
use uv::UvRenderer;
use wgpu::{CommandEncoderDescriptor, TextureView};

use super::gpu::Gpu;

pub struct Renderer {
    uv: UvRenderer,
    tractogram: TractogramRenderer,
}

impl Renderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            uv: UvRenderer::new(gpu),
            tractogram: TractogramRenderer::new(gpu),
        }
    }

    pub fn render(&self, gpu: &Gpu, view: &TextureView) {
        let mut cmd = gpu
            .device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        self.uv.render(&mut cmd, view);
        self.tractogram.render(&mut cmd, view);

        gpu.queue().submit([cmd.finish()]);
    }
}
