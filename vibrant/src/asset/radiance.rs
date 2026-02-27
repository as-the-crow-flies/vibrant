use std::{any::type_name, iter::zip};

use glam::UVec3;
use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureViewDescriptor,
    *,
};

use crate::gpu::Gpu;

struct Cascade {
    texture: Texture,
    view: TextureView,
}

impl Cascade {
    const FORMAT: TextureFormat = TextureFormat::Rgba32Float;

    pub fn new(gpu: &Gpu, size: Extent3d) -> Self {
        let texture = gpu.device().create_texture(&TextureDescriptor {
            label: Some(type_name::<Self>()),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self { texture, view }
    }

    fn view(&self) -> &TextureView {
        &self.view
    }
}

impl Drop for Cascade {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}

pub struct RadianceVolume {
    radiance: Cascade,
    binding_read: BindGroup,
    binding_write: BindGroup,

    cascades: Vec<Vec<Cascade>>,
    cascade_index_buffers: Vec<Buffer>,
    binding_cascades: Vec<BindGroup>,

    radiance_resolution: UVec3,
    cascade_resolutions: Vec<UVec3>,
}

impl RadianceVolume {
    const FORMAT: TextureFormat = TextureFormat::Rgba32Float;

    pub fn new(gpu: &Gpu, size: UVec3) -> Self {
        let label = Some(type_name::<Self>());

        let size = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: size.z,
        };

        let radiance = Cascade::new(
            gpu,
            Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: size.depth_or_array_layers,
            },
        );

        let n_cascades = 7u32;
        let n_directions = 6u32;

        let radiance_resolution = UVec3::new(size.width, size.height, size.depth_or_array_layers);
        let cascade_resolutions = (0..n_cascades)
            .map(|cascade| {
                UVec3::new(
                    size.width,
                    size.height,
                    size.depth_or_array_layers >> cascade,
                )
            })
            .collect_vec();

        let cascades = cascade_resolutions
            .iter()
            .map(|size| {
                (1..=n_directions)
                    .map(|_| {
                        Cascade::new(
                            gpu,
                            Extent3d {
                                width: size.x,
                                height: size.y,
                                depth_or_array_layers: size.z,
                            },
                        )
                    })
                    .collect_vec()
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

        let cascade_index_buffers = (0..n_cascades)
            .map(|cascade| {
                gpu.device().create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("Cascade Index {}", cascade)),
                    contents: bytemuck::bytes_of(&cascade),
                    usage: BufferUsages::UNIFORM,
                })
            })
            .collect_vec();

        let binding_cascades = zip(
            zip(cascades.iter(), cascades.iter().skip(1)),
            cascade_index_buffers.iter(),
        )
        .map(|((cascades_out, cascades_in), index)| {
            gpu.device().create_bind_group(&BindGroupDescriptor {
                label,
                layout: &Self::layout_cascade(gpu),
                entries: &[
                    vec![
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&radiance.view()),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Buffer(BufferBinding {
                                buffer: &index,
                                offset: 0,
                                size: None,
                            }),
                        },
                    ],
                    cascades_in
                        .iter()
                        .enumerate()
                        .map(|(i, cascade)| BindGroupEntry {
                            binding: (i + 3) as u32,
                            resource: BindingResource::TextureView(&cascade.view()),
                        })
                        .collect_vec(),
                    cascades_out
                        .iter()
                        .enumerate()
                        .map(|(i, cascade)| BindGroupEntry {
                            binding: (i + 9) as u32,
                            resource: BindingResource::TextureView(&cascade.view()),
                        })
                        .collect_vec(),
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
                    resource: BindingResource::TextureView(&radiance.view()),
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
            entries: &[
                [
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&radiance.view()),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&sampler),
                    },
                ]
                .to_vec(),
                cascades[0]
                    .iter()
                    .enumerate()
                    .map(|(i, cascade)| BindGroupEntry {
                        binding: (i + 2) as u32,
                        resource: BindingResource::TextureView(&cascade.view()),
                    })
                    .collect_vec(),
            ]
            .concat(),
        });

        Self {
            radiance,
            binding_read,
            binding_write,
            cascades,
            cascade_index_buffers,
            binding_cascades,
            radiance_resolution,
            cascade_resolutions,
        }
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.binding_write
    }

    pub fn binding_cascades(&self) -> &[BindGroup] {
        &self.binding_cascades
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
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    vec![
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility,
                            ty: Self::binding_type_write(),
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    (2..8)
                        .map(|i| BindGroupLayoutEntry {
                            binding: i as u32,
                            visibility,
                            ty: Self::binding_type_read(),
                            count: None,
                        })
                        .collect_vec(),
                ]
                .concat(),
            })
    }

    pub fn layout_cascade(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    vec![
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility,
                            ty: Self::binding_type_read(),
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    (3..9)
                        .map(|i| BindGroupLayoutEntry {
                            binding: i as u32,
                            visibility,
                            ty: Self::binding_type_read(),
                            count: None,
                        })
                        .collect_vec(),
                    (9..15)
                        .map(|i| BindGroupLayoutEntry {
                            binding: i as u32,
                            visibility,
                            ty: Self::binding_type_write(),
                            count: None,
                        })
                        .collect_vec(),
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

    pub fn radiance_resolution(&self) -> UVec3 {
        self.radiance_resolution
    }

    pub fn cascade_resolutions(&self) -> &[UVec3] {
        &self.cascade_resolutions
    }
}

impl Drop for RadianceVolume {
    fn drop(&mut self) {
        for buffer in &self.cascade_index_buffers {
            buffer.destroy();
        }
    }
}
