use std::{any::type_name, ops::Shr};

use glam::UVec3;
use wgpu::*;

use crate::gpu::Gpu;

pub struct GaussianRadianceBuffer {
    gaussian: Texture,
    radiance: Texture,
    binding: BindGroup,
    bindings_mipmap: Vec<BindGroup>,
    probes: Vec<UVec3>,
}

impl GaussianRadianceBuffer {
    pub const FORMAT: TextureFormat = TextureFormat::Rgba16Float;
    pub const LEVELS: u32 = 6;

    pub fn new(gpu: &Gpu, size: UVec3) -> Self {
        let label = Some(type_name::<Self>());

        let probes = (0..Self::LEVELS)
            .map(|cascade| size.shr(UVec3::splat(cascade)).max(UVec3::ONE))
            .collect();

        let descriptor = TextureDescriptor {
            label,
            size: Extent3d {
                width: size.x * 4,
                height: size.y * 4,
                depth_or_array_layers: size.z * 2,
            },
            mip_level_count: Self::LEVELS + 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let radiance = gpu.device().create_texture(&descriptor);
        let gaussian = gpu.device().create_texture(&descriptor);

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
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
                    resource: BindingResource::TextureView(&radiance.create_view(
                        &TextureViewDescriptor {
                            label,
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gaussian.create_view(
                        &TextureViewDescriptor {
                            label,
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let bindings_mipmap = (0..Self::LEVELS)
            .into_iter()
            .map(|level| {
                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout_mipmap(gpu),
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&radiance.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: level,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&gaussian.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: level,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Sampler(&sampler),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(&radiance.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: level + 1,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: BindingResource::TextureView(&gaussian.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: level + 1,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                    ],
                })
            })
            .collect();

        Self {
            radiance,
            gaussian,
            binding,
            bindings_mipmap,
            probes,
        }
    }

    pub fn size(&self) -> UVec3 {
        let s = self.gaussian().size();
        UVec3::new(s.width, s.height, s.depth_or_array_layers)
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
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
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn binding_mipmap(&self, cascade: usize) -> &BindGroup {
        &self.bindings_mipmap[cascade]
    }

    pub fn gaussian(&self) -> &Texture {
        &self.gaussian
    }

    pub fn radiance(&self) -> &Texture {
        &self.radiance
    }

    pub fn probes(&self, cascade: usize) -> UVec3 {
        self.probes[cascade]
    }
}

impl Drop for GaussianRadianceBuffer {
    fn drop(&mut self) {
        self.radiance.destroy();
        self.gaussian.destroy();
    }
}
