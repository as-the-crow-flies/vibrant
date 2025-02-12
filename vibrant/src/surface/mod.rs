pub mod color;
pub mod density;
pub mod depth;
pub mod gbuffer;
pub mod occlusion;

use color::Color;
use density::Density;
use depth::Depth;
use gbuffer::GBuffer;
use log::warn;
use occlusion::Occlusion;
use wgpu::{
    CommandEncoder, CompositeAlphaMode, Extent3d, Origin3d, PresentMode, SurfaceConfiguration,
    SurfaceTarget, TexelCopyTextureInfo, TextureAspect, TextureUsages,
};

use super::gpu::Gpu;

pub struct SurfaceBuffer {
    width: u32,
    height: u32,
    volume: u32,
    tile: u32,
    color: Color,
    depth: Depth,
    density: Density,
    occlusion: Occlusion,
    gbuffer: GBuffer,
}

impl SurfaceBuffer {
    pub fn new(gpu: &Gpu, width: u32, height: u32, volume: u32, tile: u32) -> Self {
        Self {
            width,
            height,
            volume,
            tile,
            color: Color::new(gpu, width, height),
            depth: Depth::new(gpu, width, height),
            density: Density::new(gpu, volume),
            occlusion: Occlusion::new(gpu, width.div_ceil(tile), height.div_ceil(tile), volume),
            gbuffer: GBuffer::new(gpu, width, height),
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

    pub fn tile(&self) -> u32 {
        self.tile
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

    pub fn occlusion(&self) -> &Occlusion {
        &self.occlusion
    }

    pub fn gbuffer(&self) -> &GBuffer {
        &self.gbuffer
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
            buffer: SurfaceBuffer::new(gpu, 1, 1, 1, 1),
        }
    }

    pub fn maybe_resize(
        &mut self,
        gpu: &Gpu,
        width: u32,
        height: u32,
        volume: u32,
        tile: u32,
    ) -> &Self {
        if width == self.buffer.width()
            && height == self.buffer.height()
            && volume == self.buffer.volume()
            && tile == self.buffer.tile()
        {
            return self;
        }

        self.buffer = SurfaceBuffer::new(gpu, width, height, volume, tile);
        self.surface
            .configure(gpu.device(), &Self::config(width, height));

        self
    }

    pub fn present(&self, gpu: &Gpu, mut cmd: CommandEncoder) {
        if let Some(surface) = self.surface.get_current_texture().ok() {
            cmd.copy_texture_to_texture(
                TexelCopyTextureInfo {
                    texture: self.buffer.color().texture(),
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                TexelCopyTextureInfo {
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
}
