use std::any::type_name;

use wgpu::{
    wgt::TextureViewDescriptor, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Extent3d,
    FilterMode, ShaderStages, StorageTextureAccess, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDimension,
};

use crate::{
    asset::texture::{MipTexture3D, Rgba8Unorm},
    gpu::Gpu,
};

pub struct TangentBuffer {
    ping: Texture,
    pong: Texture,
    binding: BindGroup,
    tangent: MipTexture3D<Rgba8Unorm>,
}

impl TangentBuffer {
    const FORMAT: TextureFormat = TextureFormat::Rgba32Float;

    pub fn new(gpu: &Gpu, resolution: u32) -> Self {
        let label = Some(type_name::<Self>());

        let descriptor = TextureDescriptor {
            label,
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
        };

        let ping = gpu.device().create_texture(&descriptor);
        let pong = gpu.device().create_texture(&descriptor);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&ping.create_view(
                        &TextureViewDescriptor {
                            label,
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&pong.create_view(
                        &TextureViewDescriptor {
                            label,
                            ..Default::default()
                        },
                    )),
                },
            ],
        });

        Self {
            tangent: MipTexture3D::new(gpu, resolution, FilterMode::Linear),
            ping,
            pong,
            binding,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn tangent(&self) -> &MipTexture3D<Rgba8Unorm> {
        &self.tangent
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let ty = BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: Self::FORMAT,
            view_dimension: TextureViewDimension::D3,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty,
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for TangentBuffer {
    fn drop(&mut self) {
        self.ping.destroy();
        self.pong.destroy();
    }
}
