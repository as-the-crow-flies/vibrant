use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, ColorTargetState, ColorWrites, Extent3d,
    LoadOp, Operations, RenderPassColorAttachment, ShaderStages, StorageTextureAccess, StoreOp,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct Color {
    texture: Texture,
    view: TextureView,
    view_sgrb: TextureView,
    binding_write: BindGroup,
}

impl Color {
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
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
            view_formats: &[Self::FORMAT, Self::FORMAT_SRGB],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::FORMAT),
            ..Default::default()
        });

        let view_sgrb = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::FORMAT_SRGB),
            ..Default::default()
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view),
            }],
        });

        Self {
            texture,
            view,
            view_sgrb,
            binding_write,
        }
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
            view: &self.view_sgrb,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        }
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.binding_write
    }

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: Self::FORMAT,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                }],
            })
    }
}

impl Drop for Color {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
