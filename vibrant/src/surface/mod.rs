pub mod color;
pub mod copy;
pub mod culling;
pub mod kbuffer;
pub mod occlusion;
pub mod occupancy;
pub mod opacity;
pub mod visibility;
pub mod vrc;

use std::any::type_name;

use color::ColorBuffer;
use log::warn;
use occlusion::OcclusionBuffer;
use occupancy::OccupancyBuffer;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, CommandEncoder,
    CompositeAlphaMode, PresentMode, SurfaceConfiguration, SurfaceTarget, TextureFormat,
    TextureUsages,
};

use crate::{
    asset::texture::{MipTexture2D, MipTexture3D, R32Float, R32Uint},
    controller::settings::Settings,
    surface::{
        copy::ColorCopyPipeline, culling::CullingBuffer, kbuffer::KBuffer, opacity::OpacityBuffer,
        visibility::VisibilityBuffer, vrc::VrcBuffer,
    },
};

use super::gpu::Gpu;

pub struct Frame {
    color: ColorBuffer,
    kbuffer: KBuffer,
    opacity: OpacityBuffer,
    occupancy: OccupancyBuffer,
    occlusion: OcclusionBuffer,
    culling: CullingBuffer,
    visibility: VisibilityBuffer,
    vrc: VrcBuffer,
    binding: BindGroup,
}

impl Frame {
    pub fn new(gpu: &Gpu, settings: &Settings) -> Self {
        let color = ColorBuffer::new(gpu, settings.width, settings.height);
        let kbuffer = KBuffer::new(gpu, settings.width, settings.height, 8);
        let opacity = OpacityBuffer::new(gpu, settings.width, settings.height);

        let occupancy = OccupancyBuffer::new(gpu, settings.volume);
        let occlusion = OcclusionBuffer::new(gpu, settings.volume);
        let culling = CullingBuffer::new(gpu, settings.volume);
        let visibility = VisibilityBuffer::new(gpu, settings.width, settings.height);
        let vrc = VrcBuffer::new(gpu, settings.volume);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[
                occupancy.density().binding_entries(0),
                occupancy.count().binding_entries(2),
                occlusion.ambient().binding_entries(4),
                occlusion.directional().binding_entries(6),
                opacity.opacity().binding_entries(8),
            ]
            .concat(),
        });

        Self {
            color,
            kbuffer,
            opacity,
            occupancy,
            occlusion,
            culling,
            visibility,
            vrc,
            binding,
        }
    }

    pub fn color(&self) -> &ColorBuffer {
        &self.color
    }

    pub fn kbuffer(&self) -> &KBuffer {
        &self.kbuffer
    }

    pub fn opacity(&self) -> &OpacityBuffer {
        &self.opacity
    }

    pub fn occupancy(&self) -> &OccupancyBuffer {
        &self.occupancy
    }

    pub fn occlusion(&self) -> &OcclusionBuffer {
        &self.occlusion
    }

    pub fn culling(&self) -> &CullingBuffer {
        &self.culling
    }

    pub fn visibility(&self) -> &VisibilityBuffer {
        &self.visibility
    }

    pub fn vrc(&self) -> &VrcBuffer {
        &self.vrc
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    MipTexture3D::<R32Float>::layout_entries(0), // Occupancy - Density
                    MipTexture3D::<R32Uint>::layout_entries(2),  // Occupancy - Count
                    MipTexture3D::<R32Float>::layout_entries(4), // Occlusion - Ambient
                    MipTexture3D::<R32Float>::layout_entries(6), // Occlusion - Directional
                    MipTexture2D::<R32Float>::layout_entries(8), // Opacity
                ]
                .concat(),
            })
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    buffer: Frame,
    copy: ColorCopyPipeline,
}

impl Surface {
    const FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        surface.configure(gpu.device(), &Self::config(1, 1));

        Self {
            surface,
            buffer: Frame::new(gpu, &Settings::new()),
            copy: ColorCopyPipeline::new(gpu),
        }
    }

    pub fn maybe_resize(&mut self, gpu: &Gpu, settings: &Settings) -> &Self {
        if settings.width == self.buffer.color().width()
            && settings.height == self.buffer.color().height()
            && settings.volume == self.buffer.occupancy().resolution()
        {
            return self;
        }

        self.buffer = Frame::new(gpu, &settings);
        self.surface
            .configure(gpu.device(), &Self::config(settings.width, settings.height));

        self
    }

    pub fn present(&self, gpu: &Gpu, mut cmd: CommandEncoder) {
        if let Some(surface) = self.surface.get_current_texture().ok() {
            self.copy.render(&mut cmd, &self.buffer, &surface.texture);

            gpu.submit(cmd);
            surface.present();
        } else {
            warn!("Could not obtain surface texture");
        }
    }

    fn config(width: u32, height: u32) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
            format: Self::FORMAT,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![Self::FORMAT],
        }
    }

    pub fn buffer(&self) -> &Frame {
        &self.buffer
    }
}
