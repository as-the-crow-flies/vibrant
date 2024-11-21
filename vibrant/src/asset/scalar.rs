use std::any::type_name;

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Extent3d,
    FilterMode, SamplerBindingType, SamplerDescriptor, ShaderStages, StorageTextureAccess, Texture,
    TextureAspect, TextureDescriptor, TextureFormat, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct ScalarTexture {
    texture: Texture,
    binding: BindGroup,
    binding_write: BindGroup,
    bindings_mipmap: Vec<BindGroup>,
}

impl ScalarTexture {
    const TEXTURE_FORMAT: TextureFormat = TextureFormat::R32Float;

    pub fn new(gpu: &Gpu, exponent: u32) -> Self {
        let label = Some(type_name::<Self>());
        let size = 2u32.pow(exponent);
        let mip_level_count = exponent - 2;

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: size,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: Self::TEXTURE_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
            view_formats: &[Self::TEXTURE_FORMAT],
        });

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: (exponent + 1) as f32,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::TEXTURE_FORMAT),
                            dimension: Some(TextureViewDimension::D3),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: 0,
                            array_layer_count: None,
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture.create_view(
                    &TextureViewDescriptor {
                        label,
                        format: Some(Self::TEXTURE_FORMAT),
                        dimension: Some(TextureViewDimension::D3),
                        aspect: TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: Some(1),
                        base_array_layer: 0,
                        array_layer_count: None,
                    },
                )),
            }],
        });

        let bindings_mipmap = (0..mip_level_count - 1)
            .into_iter()
            .map(|level| {
                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout_mipmap(gpu),
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&texture.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    format: Some(Self::TEXTURE_FORMAT),
                                    dimension: Some(TextureViewDimension::D3),
                                    aspect: TextureAspect::All,
                                    base_mip_level: level,
                                    mip_level_count: Some(1),
                                    base_array_layer: 0,
                                    array_layer_count: None,
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::TextureView(&texture.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    format: Some(Self::TEXTURE_FORMAT),
                                    dimension: Some(TextureViewDimension::D3),
                                    aspect: TextureAspect::All,
                                    base_mip_level: level + 1,
                                    mip_level_count: Some(1),
                                    base_array_layer: 0,
                                    array_layer_count: None,
                                },
                            )),
                        },
                    ],
                })
            })
            .collect();

        Self {
            texture,
            binding,
            binding_write,
            bindings_mipmap,
        }
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
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
                        format: Self::TEXTURE_FORMAT,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                }],
            })
    }

    pub fn layout_mipmap(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::TEXTURE_FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.binding_write
    }

    pub fn bindings_mipmap(&self) -> &[BindGroup] {
        &self.bindings_mipmap
    }
}

impl Drop for ScalarTexture {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
