pub mod buffer;
pub mod depth;
pub mod depth_hierarchy;
pub mod gbuffer;
pub mod visibility;

use buffer::FrameBuffer;
use depth::Depth;
use gbuffer::GBuffer;
use visibility::Visibility;
use wgpu::{SurfaceTarget, SurfaceTexture};

use super::gpu::Gpu;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    visibility: Visibility,
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
            visibility: Visibility::new(gpu, width, height),
            depth: Depth::new(gpu, width, height),
            gbuffer: GBuffer::new(gpu, width, height),
        }
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.surface
            .configure(gpu.device(), &FrameBuffer::configuration(width, height));

        self.visibility = Visibility::new(gpu, width, height);
        self.depth = Depth::new(gpu, width, height);
        self.gbuffer = GBuffer::new(gpu, width, height);
    }

    pub fn surface_frame(&self) -> SurfaceFrame {
        SurfaceFrame::new(
            self.surface
                .get_current_texture()
                .expect("Could not optain SurfaceTexture"),
            &self.visibility,
            &self.depth,
            &self.gbuffer,
        )
    }
}

pub struct SurfaceFrame<'a> {
    surface_texture: SurfaceTexture,
    frame: Frame<'a>,
}

impl<'a> SurfaceFrame<'a> {
    pub fn new(
        surface_texture: SurfaceTexture,
        visibility: &'a Visibility,
        depth: &'a Depth,
        gbuffer: &'a GBuffer,
    ) -> Self {
        Self {
            frame: Frame::new(
                surface_texture.texture.width(),
                surface_texture.texture.height(),
                FrameBuffer::new(&surface_texture.texture),
                visibility,
                depth,
                gbuffer,
            ),
            surface_texture,
        }
    }

    pub fn present(self) {
        self.surface_texture.present();
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }
}

pub struct Frame<'a> {
    width: u32,
    height: u32,
    frame_buffer: FrameBuffer,
    visibility: &'a Visibility,
    depth: &'a Depth,
    gbuffer: &'a GBuffer,
}

impl<'a> Frame<'a> {
    pub fn new(
        width: u32,
        height: u32,
        frame_buffer: FrameBuffer,
        visibility: &'a Visibility,
        depth: &'a Depth,
        gbuffer: &'a GBuffer,
    ) -> Self {
        Frame {
            width,
            height,
            frame_buffer,
            visibility,
            depth,
            gbuffer,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn buffer(&self) -> &FrameBuffer {
        &self.frame_buffer
    }

    pub fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    pub fn depth(&self) -> &Depth {
        &self.depth
    }

    pub fn gbuffer(&self) -> &GBuffer {
        &self.gbuffer
    }
}
