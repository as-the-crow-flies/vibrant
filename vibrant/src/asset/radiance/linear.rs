use std::any::type_name;

use bytemuck::bytes_of;
use glam::UVec3;
use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    BufferBindingType, BufferUsages, Extent3d, FilterMode, MipmapFilterMode, SamplerBindingType,
    SamplerDescriptor, ShaderStages, StorageTextureAccess, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct LinearRadianceBuffer {
    irradiance: Texture,
    radiance: Vec<Texture>,
    transmission: Vec<Texture>,
    importance: Vec<Texture>,
    binding_cascade: Vec<BindGroup>,
    binding_read: BindGroup,
    binding_copy: BindGroup,
    size: Extent3d,
    size_cascade: Vec<Extent3d>,
}

impl LinearRadianceBuffer {
    pub const N_CASCADES: usize = 9;
    pub const FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

    pub fn new(gpu: &Gpu, volume: UVec3) -> Self {
        let label = Some(type_name::<Self>());

        let size = Extent3d {
            width: volume.x,
            height: volume.y,
            depth_or_array_layers: volume.z,
        };

        let descriptor = TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let irradiance = gpu.device().create_texture(&descriptor);

        let mut probes = volume;
        let mut samples = 1;

        let size_cascade = (0..Self::N_CASCADES)
            .map(|_| {
                let extend = Extent3d {
                    width: probes.x.max(1) * samples,
                    height: probes.y.max(1) * samples,
                    depth_or_array_layers: probes.z.max(1),
                };

                probes /= 2;
                samples *= 2;

                extend
            })
            .collect_vec();

        let radiance = (0..Self::N_CASCADES)
            .map(|cascade| {
                gpu.device().create_texture(&TextureDescriptor {
                    size: size_cascade[cascade],
                    ..descriptor
                })
            })
            .collect_vec();

        let transmission = (0..Self::N_CASCADES)
            .map(|cascade| {
                gpu.device().create_texture(&TextureDescriptor {
                    size: size_cascade[cascade],
                    ..descriptor
                })
            })
            .collect_vec();

        let importance = (0..Self::N_CASCADES)
            .map(|cascade| {
                gpu.device().create_texture(&TextureDescriptor {
                    size: size_cascade[cascade],
                    ..descriptor
                })
            })
            .collect_vec();

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let binding_cascade = (0..Self::N_CASCADES)
            .map(|index| {
                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout_cascade(gpu),
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(
                                &radiance[index].create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(
                                &transmission[index].create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::TextureView(
                                &importance[index].create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(
                                &radiance[(index + 1) % Self::N_CASCADES]
                                    .create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: BindingResource::TextureView(
                                &irradiance.create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: gpu
                                .device()
                                .create_buffer_init(&BufferInitDescriptor {
                                    label,
                                    contents: bytes_of(&(index as u32)),
                                    usage: BufferUsages::UNIFORM,
                                })
                                .as_entire_binding(),
                        },
                    ],
                })
            })
            .collect_vec();

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &irradiance.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &radiance[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(
                        &radiance[1].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &radiance[2].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(
                        &radiance[3].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(
                        &radiance[4].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(
                        &radiance[5].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(
                        &radiance[6].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(
                        &radiance[7].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(
                        &radiance[8].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(
                        &transmission[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureView(
                        &transmission[1].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::TextureView(
                        &transmission[2].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::TextureView(
                        &transmission[3].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 15,
                    resource: BindingResource::TextureView(
                        &transmission[4].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 16,
                    resource: BindingResource::TextureView(
                        &transmission[5].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 17,
                    resource: BindingResource::TextureView(
                        &transmission[6].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 18,
                    resource: BindingResource::TextureView(
                        &transmission[7].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 19,
                    resource: BindingResource::TextureView(
                        &transmission[8].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 20,
                    resource: BindingResource::TextureView(
                        &importance[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 21,
                    resource: BindingResource::TextureView(
                        &importance[1].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 22,
                    resource: BindingResource::TextureView(
                        &importance[2].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 23,
                    resource: BindingResource::TextureView(
                        &importance[3].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 24,
                    resource: BindingResource::TextureView(
                        &importance[4].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 25,
                    resource: BindingResource::TextureView(
                        &importance[5].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 26,
                    resource: BindingResource::TextureView(
                        &importance[6].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 27,
                    resource: BindingResource::TextureView(
                        &importance[7].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 28,
                    resource: BindingResource::TextureView(
                        &importance[8].create_view(&TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        let binding_copy = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_copy(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &irradiance.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &radiance[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        Self {
            irradiance,
            radiance,
            transmission,
            importance,
            binding_cascade,
            binding_read,
            binding_copy,
            size,
            size_cascade,
        }
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    vec![BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    }],
                    (1..29)
                        .map(|binding| BindGroupLayoutEntry {
                            binding,
                            visibility,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        })
                        .collect_vec(),
                ]
                .concat(),
            })
    }

    pub fn layout_copy(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Sampler
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Irradiance
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    // Radiance 0
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
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

    pub fn layout_cascade(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Radiance 0
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    // Transmission 0
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    // Importance 0
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    // Radiance 1
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Irradiance
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // CASCADE
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn binding_cascade(&self) -> &[BindGroup] {
        &self.binding_cascade
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_copy(&self) -> &BindGroup {
        &self.binding_copy
    }

    pub fn size_cascade(&self) -> &[Extent3d] {
        &self.size_cascade
    }

    pub fn size(&self) -> Extent3d {
        self.size
    }
}

impl Drop for LinearRadianceBuffer {
    fn drop(&mut self) {
        self.irradiance.destroy();

        for radiance in &self.radiance {
            radiance.destroy();
        }

        for transmission in &self.transmission {
            transmission.destroy();
        }

        for importance in &self.importance {
            importance.destroy();
        }
    }
}
