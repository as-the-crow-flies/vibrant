pub mod buffer;
pub mod depth;
pub mod gbuffer;
pub mod hierarchy;
pub mod visibility;

use buffer::FrameBuffer;
use depth::Depth;
use gbuffer::GBuffer;
use hierarchy::DepthHierarchy;
use visibility::Visibility;
use wgpu::{SurfaceTarget, SurfaceTexture, Texture};

use super::gpu::Gpu;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    visibility: Visibility,
    depth: Depth,
    gbuffer: GBuffer,
    hierarchy: DepthHierarchy,
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
            visibility: Visibility::new(gpu, width, height),
            depth: Depth::new(gpu, width, height),
            gbuffer: GBuffer::new(gpu, width, height),
            hierarchy: DepthHierarchy::new(gpu, width / 4, height / 4),
        }
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.surface
            .configure(gpu.device(), &FrameBuffer::configuration(width, height));

        self.visibility = Visibility::new(gpu, width, height);
        self.depth = Depth::new(gpu, width, height);
        self.gbuffer = GBuffer::new(gpu, width, height);
        self.hierarchy = DepthHierarchy::new(gpu, width / 4, height / 4);
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
                visibility: &self.visibility,
                depth: &self.depth,
                gbuffer: &self.gbuffer,
                hierarchy: &self.hierarchy,
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
    pub visibility: &'a Visibility,
    pub depth: &'a Depth,
    pub gbuffer: &'a GBuffer,
    pub hierarchy: &'a DepthHierarchy,
}
