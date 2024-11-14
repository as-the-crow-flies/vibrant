use std::any::type_name;

use wgpu::{
    Color, ColorTargetState, ColorWrites, CompositeAlphaMode, LoadOp, Operations, PresentMode,
    RenderPassColorAttachment, StoreOp, SurfaceConfiguration, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::gpu::Gpu;

pub struct FrameBuffer {
    view: TextureView,
    view_srgb: TextureView,
}

impl FrameBuffer {
    pub const FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
    pub const FORMAT_SRGB: TextureFormat = TextureFormat::Bgra8UnormSrgb;

    pub fn new(texture: &Texture) -> Self {
        let label = Some(type_name::<Self>());

        Self {
            view: texture.create_view(&TextureViewDescriptor {
                label,
                format: Some(Self::FORMAT),
                ..Default::default()
            }),
            view_srgb: texture.create_view(&TextureViewDescriptor {
                label,
                format: Some(Self::FORMAT_SRGB),
                ..Default::default()
            }),
        }
    }

    pub fn configuration(width: u32, height: u32) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: Self::FORMAT,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 3,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![Self::FORMAT, Self::FORMAT_SRGB],
        }
    }

    pub fn target() -> ColorTargetState {
        ColorTargetState {
            format: Self::FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn target_srgb() -> ColorTargetState {
        ColorTargetState {
            format: Self::FORMAT_SRGB,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn attachment(&self) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: &self.view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        }
    }

    pub fn attachment_srgb(&self) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: &self.view_srgb,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::BLACK),
                store: StoreOp::Store,
            },
        }
    }

    pub fn texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
        gpu.device().create_texture(&TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[Self::FORMAT, Self::FORMAT_SRGB],
        })
    }
}
