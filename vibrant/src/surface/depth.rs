use std::any::type_name;

use wgpu::{
    CompareFunction, DepthBiasState, DepthStencilState, Extent3d, LoadOp, Operations,
    RenderPassDepthStencilAttachment, StencilState, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::gpu::Gpu;

pub struct Depth {
    texture: Texture,
    view: TextureView,
}

impl Depth {
    pub const FORMAT: TextureFormat = TextureFormat::Depth32Float;

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
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[Self::FORMAT],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::FORMAT),
            ..Default::default()
        });

        Self { texture, view }
    }

    pub fn attachment(&self) -> RenderPassDepthStencilAttachment {
        RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    pub fn state() -> DepthStencilState {
        DepthStencilState {
            format: Self::FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }
    }
}

impl Drop for Depth {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
