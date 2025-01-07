pub mod color;
pub mod density;
pub mod depth;
pub mod gbuffer;
pub mod kbuffer;

use color::Color;
use density::Density;
use depth::Depth;
use gbuffer::GBuffer;
use kbuffer::KBuffer;
use log::warn;
use wgpu::{
    CommandEncoder, CompositeAlphaMode, Extent3d, ImageCopyTexture, Origin3d, PresentMode,
    SurfaceConfiguration, SurfaceTarget, TextureAspect, TextureUsages,
};

use crate::asset::scalar::ScalarTexture;

use super::gpu::Gpu;

pub struct SurfaceBuffer {
    width: u32,
    height: u32,
    volume: u32,
    color: Color,
    depth: Depth,
    density: Density,
    occlusion: ScalarTexture,
    gbuffer: GBuffer,
    kbuffer: KBuffer,
}

impl SurfaceBuffer {
    pub fn new(gpu: &Gpu, width: u32, height: u32, volume: u32) -> Self {
        Self {
            width,
            height,
            volume,
            density: Density::new(gpu, volume, volume, volume),
            occlusion: ScalarTexture::new(gpu, volume, volume, volume),
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

    pub fn volume(&self) -> u32 {
        self.volume
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn depth(&self) -> &Depth {
        &self.depth
    }

    pub fn density(&self) -> &Density {
        &self.density
    }

    pub fn occlusion(&self) -> &ScalarTexture {
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
    width: u32,
    height: u32,
    volume: u32,
    surface: wgpu::Surface<'static>,
    buffer: SurfaceBuffer,
}

impl Surface {
    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let width = 1;
        let height = 1;
        let volume = 1;

        surface.configure(gpu.device(), &Self::config(width, height));

        Self {
            width,
            height,
            volume,
            surface,
            buffer: SurfaceBuffer::new(gpu, width, height, volume),
        }
    }

    pub fn maybe_resize(&mut self, gpu: &Gpu, width: u32, height: u32, volume: u32) -> &Self {
        if width == self.width && height == self.height && volume == self.volume {
            return self;
        }

        self.width = width;
        self.height = height;
        self.volume = volume;

        self.surface
            .configure(gpu.device(), &Self::config(width, height));
        self.buffer = SurfaceBuffer::new(gpu, width, height, volume);

        self
    }

    pub fn present(&self, gpu: &Gpu, mut cmd: CommandEncoder) {
        if let Some(surface) = self.surface.get_current_texture().ok() {
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
        } else {
            warn!("Could not obtain surface texture");
        }
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

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn volume(&self) -> u32 {
        self.volume
    }
}
