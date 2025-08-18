use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Extent3d, ShaderStages, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct VisibilityBuffer {
    depth: Texture,
    depth_view: TextureView,
    index: Texture,
    index_view: TextureView,
    binding: BindGroup,
}

impl VisibilityBuffer {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
    pub const INDEX_FORMAT: TextureFormat = TextureFormat::R32Uint;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let depth = gpu.device().create_texture(&TextureDescriptor {
            label: Some("VisibilityBuffer::Depth"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth.create_view(&TextureViewDescriptor::default());

        let index = gpu.device().create_texture(&TextureDescriptor {
            label: Some("VisibilityBuffer::Index"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::INDEX_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let index_view = index.create_view(&TextureViewDescriptor::default());

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&depth_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&index_view),
                },
            ],
        });

        Self {
            depth,
            depth_view,
            index,
            index_view,
            binding,
        }
    }

    pub fn depth_view(&self) -> &TextureView {
        &self.depth_view
    }

    pub fn index_view(&self) -> &TextureView {
        &self.index_view
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
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for VisibilityBuffer {
    fn drop(&mut self) {
        self.depth.destroy();
        self.index.destroy();
    }
}
