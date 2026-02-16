use std::any::type_name;

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Extent3d,
    FilterMode, SamplerBindingType, SamplerDescriptor, ShaderStages, StorageTextureAccess, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct BloomBuffer {
    texture_a: Texture,
    texture_b: Texture,
    read_a: BindGroup,
    read_b: BindGroup,
    write_a: BindGroup,
    write_b: BindGroup,
}

impl BloomBuffer {
    pub const FORMAT: TextureFormat = TextureFormat::Rgba16Float;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let label = Some(type_name::<Self>());

        let w = (width / 2).max(1);
        let h = (height / 2).max(1);

        let create_texture = || {
            gpu.device().create_texture(&TextureDescriptor {
                label,
                size: Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: Self::FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[Self::FORMAT],
            })
        };

        let texture_a = create_texture();
        let texture_b = create_texture();

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let view_a = texture_a.create_view(&TextureViewDescriptor::default());
        let view_b = texture_b.create_view(&TextureViewDescriptor::default());

        let read_a = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::read_layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view_a),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let read_b = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::read_layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view_b),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let write_a = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::write_layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view_a),
            }],
        });

        let write_b = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::write_layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view_b),
            }],
        });

        Self {
            texture_a,
            texture_b,
            read_a,
            read_b,
            write_a,
            write_b,
        }
    }

    pub fn width(&self) -> u32 {
        self.texture_a.width()
    }

    pub fn height(&self) -> u32 {
        self.texture_a.height()
    }

    pub fn read_a(&self) -> &BindGroup {
        &self.read_a
    }

    pub fn read_b(&self) -> &BindGroup {
        &self.read_b
    }

    pub fn write_a(&self) -> &BindGroup {
        &self.write_a
    }

    pub fn write_b(&self) -> &BindGroup {
        &self.write_b
    }

    pub fn read_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    pub fn write_layout(gpu: &Gpu) -> BindGroupLayout {
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

impl Drop for BloomBuffer {
    fn drop(&mut self) {
        self.texture_a.destroy();
        self.texture_b.destroy();
    }
}
