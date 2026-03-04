use std::{any::type_name, iter::zip, num::NonZero};

use glam::UVec3;
use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::gpu::Gpu;

pub struct Cascade {
    radiance: Vec<Texture>,
    radiance_views: Vec<TextureView>,
    transmission: Vec<Texture>,
    transmission_views: Vec<TextureView>,
    size: UVec3,
}

impl Cascade {
    pub fn new(gpu: &Gpu, size: UVec3, format: TextureFormat, directions: u32) -> Self {
        let descriptor = TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: size.z,
            },
            mip_level_count: 1,
            sample_count: 1,
            format,
            dimension: TextureDimension::D3,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let radiance = (0..directions)
            .map(|_| gpu.device().create_texture(&descriptor))
            .collect_vec();

        let radiance_views = radiance
            .iter()
            .map(|texture| texture.create_view(&TextureViewDescriptor::default()))
            .collect_vec();

        let transmission = (0..directions)
            .map(|_| gpu.device().create_texture(&descriptor))
            .collect_vec();

        let transmission_views = transmission
            .iter()
            .map(|texture| texture.create_view(&TextureViewDescriptor::default()))
            .collect_vec();

        Self {
            radiance,
            radiance_views,
            transmission,
            transmission_views,
            size,
        }
    }

    pub fn radiance_views(&self) -> Vec<&TextureView> {
        self.radiance_views.iter().collect_vec()
    }

    pub fn transmission_views(&self) -> Vec<&TextureView> {
        self.transmission_views.iter().collect_vec()
    }

    pub fn size(&self) -> UVec3 {
        self.size
    }
}

impl Drop for Cascade {
    fn drop(&mut self) {
        for texture in &self.radiance {
            texture.destroy();
        }

        for texture in &self.transmission {
            texture.destroy();
        }
    }
}

pub struct RadianceVolume {
    cascades: Vec<Cascade>,
    cascade_indices: Vec<Buffer>,
    cascade_bindings: Vec<BindGroup>,

    binding_read: BindGroup,
}

impl RadianceVolume {
    const FORMAT: TextureFormat = TextureFormat::Rgba16Float;
    const N_CASCADES: u32 = 7;
    const N_DIRECTIONS: u32 = 6;

    pub fn new(gpu: &Gpu, size: UVec3) -> Self {
        let label = Some(type_name::<Self>());

        let cascades = (0..Self::N_CASCADES)
            .map(|cascade| {
                Cascade::new(
                    gpu,
                    UVec3::new(size.x, size.y, size.z >> cascade),
                    Self::FORMAT,
                    Self::N_DIRECTIONS,
                )
            })
            .collect_vec();

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let cascade_indices = (0..Self::N_CASCADES)
            .map(|cascade| {
                gpu.device().create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("Cascade Index {}", cascade)),
                    contents: bytemuck::bytes_of(&cascade),
                    usage: BufferUsages::UNIFORM,
                })
            })
            .collect_vec();

        let cascade_bindings = zip(
            zip(cascades.iter(), cascades.iter().skip(1)),
            &cascade_indices,
        )
        .map(|((cascade_out, cascade_in), index)| {
            gpu.device().create_bind_group(&BindGroupDescriptor {
                label,
                layout: &Self::layout_cascade(gpu),
                entries: &[
                    cascade_out
                        .radiance_views()
                        .iter()
                        .enumerate()
                        .map(|(index, view)| BindGroupEntry {
                            binding: index as u32,
                            resource: BindingResource::TextureView(view),
                        })
                        .collect_vec(),
                    cascade_out
                        .transmission_views()
                        .iter()
                        .enumerate()
                        .map(|(index, view)| BindGroupEntry {
                            binding: Self::N_DIRECTIONS + index as u32,
                            resource: BindingResource::TextureView(view),
                        })
                        .collect_vec(),
                    cascade_in
                        .radiance_views()
                        .iter()
                        .enumerate()
                        .map(|(index, view)| BindGroupEntry {
                            binding: 2 * Self::N_DIRECTIONS + index as u32,
                            resource: BindingResource::TextureView(view),
                        })
                        .collect_vec(),
                    vec![
                        BindGroupEntry {
                            binding: 3 * Self::N_DIRECTIONS,
                            resource: BindingResource::Sampler(&sampler),
                        },
                        BindGroupEntry {
                            binding: 3 * Self::N_DIRECTIONS + 1,
                            resource: BindingResource::Buffer(BufferBinding {
                                buffer: &index,
                                offset: 0,
                                size: None,
                            }),
                        },
                    ],
                ]
                .concat(),
            })
        })
        .collect_vec();

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureViewArray(&cascades[0].radiance_views()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureViewArray(&cascades[1].radiance_views()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureViewArray(&cascades[2].radiance_views()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureViewArray(&cascades[3].radiance_views()),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureViewArray(&cascades[4].radiance_views()),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureViewArray(&cascades[5].radiance_views()),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureViewArray(&cascades[0].transmission_views()),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureViewArray(&cascades[1].transmission_views()),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureViewArray(&cascades[2].transmission_views()),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureViewArray(&cascades[3].transmission_views()),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureViewArray(&cascades[4].transmission_views()),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureViewArray(&cascades[5].transmission_views()),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            binding_read,
            cascades,
            cascade_indices,
            cascade_bindings,
        }
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_cascades(&self) -> &[BindGroup] {
        &self.cascade_bindings
    }

    pub fn cascades(&self) -> &[Cascade] {
        &self.cascades
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: NonZero::new(6),
                    },
                    BindGroupLayoutEntry {
                        binding: 12,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    pub fn layout_cascade(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    (0..2 * Self::N_DIRECTIONS)
                        .map(|index| BindGroupLayoutEntry {
                            binding: index as u32,
                            visibility,
                            ty: Self::binding_type_write(),
                            count: None,
                        })
                        .collect_vec(),
                    (2 * Self::N_DIRECTIONS..3 * Self::N_DIRECTIONS)
                        .map(|index| BindGroupLayoutEntry {
                            binding: index as u32,
                            visibility,
                            ty: Self::binding_type_read(),
                            count: None,
                        })
                        .collect_vec(),
                    vec![
                        BindGroupLayoutEntry {
                            binding: 3 * Self::N_DIRECTIONS,
                            visibility,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 3 * Self::N_DIRECTIONS + 1,
                            visibility,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                ]
                .concat(),
            })
    }

    fn binding_type_read() -> BindingType {
        BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::D3,
            multisampled: false,
        }
    }

    fn binding_type_write() -> BindingType {
        BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format: Self::FORMAT,
            view_dimension: TextureViewDimension::D3,
        }
    }
}

impl Drop for RadianceVolume {
    fn drop(&mut self) {
        for buffer in &self.cascade_indices {
            buffer.destroy();
        }
    }
}
