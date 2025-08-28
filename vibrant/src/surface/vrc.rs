use std::any::type_name;

use wgpu::{
    wgt::TextureViewDescriptor, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Extent3d,
    ShaderStages, StorageTextureAccess, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct VrcBuffer {
    start: Texture,
    end: Texture,
    binding: BindGroup,
}

impl VrcBuffer {
    const FORMAT: TextureFormat = TextureFormat::R32Uint;

    pub fn new(gpu: &Gpu, resolution: u32) -> Self {
        let start = gpu.device().create_texture(&TextureDescriptor {
            label: Some("VrcBuffer::Start"),
            size: Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: resolution,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let start_view = start.create_view(&TextureViewDescriptor::default());

        let end = gpu.device().create_texture(&TextureDescriptor {
            label: Some("VrcBuffer::Start"),
            size: Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: resolution,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let end_view = end.create_view(&TextureViewDescriptor::default());

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&start_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&end_view),
                },
            ],
        });

        Self {
            start,
            end,
            binding,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for VrcBuffer {
    fn drop(&mut self) {
        self.start.destroy();
        self.end.destroy();
    }
}
