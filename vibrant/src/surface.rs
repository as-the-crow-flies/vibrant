pub mod buffer;
pub mod depth;
pub mod gbuffer;

use buffer::FrameBuffer;
use depth::Depth;
use gbuffer::GBuffer;
use wgpu::{SurfaceTarget, SurfaceTexture, Texture};

use super::gpu::Gpu;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    depth: Depth,
    gbuffer: GBuffer,
}

impl Surface {
    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let (width, height) = (1, 1);

        surface.configure(gpu.device(), &FrameBuffer::configuration(width, height));

        Self {
            surface,
            depth: Depth::new(gpu, width, height),
            gbuffer: GBuffer::new(gpu, width, height),
        }
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.surface
            .configure(gpu.device(), &FrameBuffer::configuration(width, height));
        self.depth = Depth::new(gpu, width, height);
        self.gbuffer = GBuffer::new(gpu, width, height);
    }

    pub fn surface_frame(&self) -> SurfaceFrame {
        let texture = self
            .surface
            .get_current_texture()
            .expect("Could not optain SurfaceTexture");

        SurfaceFrame {
            frame: Frame {
                width: texture.texture.width(),
                height: texture.texture.height(),
                buffer: FrameBuffer::new(&texture.texture),
                depth: &self.depth,
                gbuffer: &self.gbuffer,
            },
            texture,
        }
    }
}

pub struct SurfaceFrame<'a> {
    texture: SurfaceTexture,
    frame: Frame<'a>,
}

impl<'a> SurfaceFrame<'a> {
    pub fn present(self) {
        self.texture.present();
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn texture(&self) -> &Texture {
        &self.texture.texture
    }
}

pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    pub buffer: FrameBuffer,
    pub depth: &'a Depth,
    pub gbuffer: &'a GBuffer,
}
