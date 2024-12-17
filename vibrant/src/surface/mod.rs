pub mod color;
pub mod depth;
pub mod gbuffer;
pub mod kbuffer;
pub mod occlusion;

use color::Color;
use depth::Depth;
use gbuffer::GBuffer;
use kbuffer::KBuffer;
use occlusion::Occlusion;
use wgpu::{
    CommandEncoder, CompositeAlphaMode, Extent3d, ImageCopyTexture, Origin3d, PresentMode,
    SurfaceConfiguration, SurfaceTarget, TextureAspect, TextureUsages,
};

use crate::constants::{OCCLUSION_DEPTH, TILE_SIZE};

use super::gpu::Gpu;

pub struct SurfaceBuffer {
    width: u32,
    height: u32,
    color: Color,
    depth: Depth,
    occlusion: Occlusion,
    gbuffer: GBuffer,
    kbuffer: KBuffer,
}

impl SurfaceBuffer {
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            occlusion: Occlusion::new(
                gpu,
                width.div_ceil(TILE_SIZE),
                height.div_ceil(TILE_SIZE),
                OCCLUSION_DEPTH,
            ),
            color: Color::new(gpu, width, height),
            depth: Depth::new(gpu, width, height),
            gbuffer: GBuffer::new(gpu, width, height),
            kbuffer: KBuffer::new(gpu, width, height),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn depth(&self) -> &Depth {
        &self.depth
    }

    pub fn density(&self) -> &Occlusion {
        &self.occlusion
    }

    pub fn gbuffer(&self) -> &GBuffer {
        &self.gbuffer
    }

    pub fn kbuffer(&self) -> &KBuffer {
        &self.kbuffer
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    buffer: SurfaceBuffer,
}

impl Surface {
    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        surface.configure(gpu.device(), &Self::config(1, 1));

        Self {
            surface,
            buffer: SurfaceBuffer::new(gpu, 1, 1),
        }
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.surface
            .configure(gpu.device(), &Self::config(width, height));
        self.buffer = SurfaceBuffer::new(gpu, width, height)
    }

    pub fn present(&self, gpu: &Gpu, mut cmd: CommandEncoder) {
        let surface = self
            .surface
            .get_current_texture()
            .expect("Could not optain SurfaceTexture");

        cmd.copy_texture_to_texture(
            ImageCopyTexture {
                texture: self.buffer.color().texture(),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &surface.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: surface.texture.width(),
                height: surface.texture.height(),
                depth_or_array_layers: 1,
            },
        );

        gpu.submit(cmd);
        surface.present();
    }

    fn config(width: u32, height: u32) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
            format: Color::FORMAT,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![Color::FORMAT],
        }
    }

    pub fn buffer(&self) -> &SurfaceBuffer {
        &self.buffer
    }
}
