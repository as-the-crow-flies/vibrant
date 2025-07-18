use std::any::type_name;

use wgpu::{
    ColorTargetState, ColorWrites, Extent3d, LoadOp, Operations, RenderPassColorAttachment,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};

use crate::gpu::Gpu;

pub struct ColorBuffer {
    texture: Texture,
    view: TextureView,
    view_srgb: TextureView,
}

impl ColorBuffer {
    pub const FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
    pub const FORMAT_SRGB: TextureFormat = TextureFormat::Bgra8UnormSrgb;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let label = Some(type_name::<Self>());

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
            view_formats: &[Self::FORMAT, Self::FORMAT_SRGB],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::FORMAT),
            ..Default::default()
        });

        let view_srgb = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::FORMAT_SRGB),
            ..Default::default()
        });

        Self {
            texture,
            view,
            view_srgb,
        }
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
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
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        }
    }
}

impl Drop for ColorBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
