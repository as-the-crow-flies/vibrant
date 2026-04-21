use std::{any::type_name, iter::zip};

use glam::UVec3;
use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureViewDescriptor,
    *,
};

use crate::gpu::Gpu;

pub struct Cascade {
    radiance: Texture,
    radiance_view: TextureView,
    transmission: Texture,
    transmission_view: TextureView,
    size: UVec3,
}

impl Cascade {
    pub fn new(gpu: &Gpu, size: UVec3, format: TextureFormat) -> Self {
        let descriptor = TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: size.z * 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            format,
            dimension: TextureDimension::D3,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let radiance = gpu.device().create_texture(&descriptor);
        let radiance_view = radiance.create_view(&TextureViewDescriptor::default());

        let transmission = gpu.device().create_texture(&descriptor);
        let transmission_view = transmission.create_view(&TextureViewDescriptor::default());

        Self {
            radiance,
            radiance_view,
            transmission,
            transmission_view,
            size,
        }
    }

    pub fn size(&self) -> UVec3 {
        self.size
    }
}

impl Drop for Cascade {
    fn drop(&mut self) {
        self.radiance.destroy();
        self.transmission.destroy();
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

    pub fn new(gpu: &Gpu, size: UVec3) -> Self {
        let label = Some(type_name::<Self>());

        let cascades = (1..=Self::N_CASCADES)
            .map(|cascade| {
                Cascade::new(
                    gpu,
                    UVec3::new(size.x >> 1, size.y >> 1, size.z >> cascade),
                    Self::FORMAT,
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
            mipmap_filter: MipmapFilterMode::Linear,
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
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&cascade_out.radiance_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&cascade_out.transmission_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&cascade_in.radiance_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::Sampler(&sampler),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &index,
                            offset: 0,
                            size: None,
                        }),
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
                    resource: BindingResource::TextureView(&cascades[0].radiance_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&cascades[1].radiance_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&cascades[2].radiance_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&cascades[3].radiance_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&cascades[4].radiance_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&cascades[5].radiance_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&cascades[0].transmission_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&cascades[1].transmission_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&cascades[2].transmission_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&cascades[3].transmission_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&cascades[4].transmission_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(&cascades[5].transmission_view),
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
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
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
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: Self::binding_type_write(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: Self::binding_type_write(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
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
