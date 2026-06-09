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

pub struct OctahedralRadianceCascadesBuffer {
    irradiance: Texture,
    radiance: Vec<Texture>,
    transmission: Vec<Texture>,
    binding_cascade: Vec<BindGroup>,
    binding_read: BindGroup,
    binding_copy: BindGroup,
    size: Extent3d,
    size_cascade: Vec<Extent3d>,
}

impl OctahedralRadianceCascadesBuffer {
    pub const N_CASCADES: usize = 6;
    pub const FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

    pub fn new(gpu: &Gpu, size: UVec3) -> Self {
        let label = Some(type_name::<Self>());

        let size = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: size.z,
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

        let size_cascade = (0..Self::N_CASCADES)
            .map(|cascade| Extent3d {
                width: size.width * 2,
                height: size.height * 2,
                depth_or_array_layers: ((size.depth_or_array_layers >> cascade).max(1)) * 2,
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
                                &radiance[(index + 1) % Self::N_CASCADES]
                                    .create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(
                                &irradiance.create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 4,
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
                    resource: BindingResource::TextureView(
                        &irradiance.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &radiance[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &radiance[1].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(
                        &radiance[2].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &radiance[3].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(
                        &radiance[4].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(
                        &radiance[5].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(
                        &transmission[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(
                        &transmission[1].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(
                        &transmission[2].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(
                        &transmission[3].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(
                        &transmission[4].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureView(
                        &transmission[5].create_view(&TextureViewDescriptor::default()),
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
                    resource: BindingResource::TextureView(
                        &irradiance.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &radiance[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            irradiance,
            radiance,
            transmission,
            binding_cascade,
            binding_read,
            binding_copy,
            size,
            size_cascade,
        }
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &(0..13)
                    .map(|binding| BindGroupLayoutEntry {
                        binding,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    })
                    .collect_vec(),
            })
    }

    pub fn layout_copy(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Irradiance
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
                    // Radiance 0
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
                    // Sampler
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
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
                    // Radiance 1
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
                    // Irradiance
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
                    BindGroupLayoutEntry {
                        binding: 4,
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

impl Drop for OctahedralRadianceCascadesBuffer {
    fn drop(&mut self) {
        self.irradiance.destroy();

        for radiance in &self.radiance {
            radiance.destroy();
        }

        for transmission in &self.transmission {
            transmission.destroy();
        }
    }
}
