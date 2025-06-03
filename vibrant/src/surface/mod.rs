pub mod color;
pub mod density;
pub mod occlusion;
pub mod occupancy;

use color::Color;
use density::Density;
use log::warn;
use occlusion::Occlusion;
use wgpu::{
    CommandEncoder, CompositeAlphaMode, Extent3d, Origin3d, PresentMode, SurfaceConfiguration,
    SurfaceTarget, TexelCopyTextureInfo, TextureAspect, TextureUsages,
};

use crate::{controller::Controller, surface::occupancy::Occupancy};

use super::gpu::Gpu;

pub struct SurfaceBuffer {
    color: Color,
    density: Density,
    occlusion: Occlusion,
    occupancy: Occupancy,
}

impl SurfaceBuffer {
    pub fn new(gpu: &Gpu, controller: &Controller) -> Self {
        Self {
            color: Color::new(gpu, controller.width(), controller.height()),
            density: Density::new(gpu, controller.density()),
            occlusion: Occlusion::new(gpu, controller.occlusion()),
            occupancy: Occupancy::new(gpu, controller.density(), controller.memory()),
        }
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn density(&self) -> &Density {
        &self.density
    }

    pub fn occlusion(&self) -> &Occlusion {
        &self.occlusion
    }

    pub fn occupancy(&self) -> &Occupancy {
        &self.occupancy
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
            buffer: SurfaceBuffer::new(gpu, &Controller::test(1, 1, 1, 1, 1)),
        }
    }

    pub fn maybe_resize(&mut self, gpu: &Gpu, controller: &Controller) -> &Self {
        if controller.width() == self.buffer.color().width()
            && controller.height() == self.buffer.color().height()
            && controller.density() == self.buffer.density().resolution()
            && controller.occlusion() == self.buffer.occlusion().resolution()
        {
            return self;
        }

        self.buffer = SurfaceBuffer::new(gpu, &controller);
        self.surface.configure(
            gpu.device(),
            &Self::config(controller.width(), controller.height()),
        );

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
