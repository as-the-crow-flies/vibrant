use std::{any::type_name, iter::zip, ops::Add};

use glam::UVec3;
use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::gpu::Gpu;

pub struct RadianceVolume {
    radiance: Texture,
    binding_read: BindGroup,
    binding_write: BindGroup,

    cascades: Vec<Texture>,
    cascade_index_buffers: Vec<Buffer>,
    binding_cascades: Vec<BindGroup>,
}

impl RadianceVolume {
    const FORMAT: TextureFormat = TextureFormat::Rgba16Float;

    pub fn new(gpu: &Gpu, size: Extent3d) -> Self {
        let label = Some(type_name::<Self>());

        let radiance = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: size.depth_or_array_layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        // let n_cascades = size
        //     .width
        //     .min(size.height)
        //     .min(size.depth_or_array_layers)
        //     .ilog(2);

        let n_cascades = 7u32;

        dbg!(radiance.size());
        dbg!(n_cascades);

        let cascades = (1..=n_cascades)
            .map(|cascade| {
                gpu.device().create_texture(&TextureDescriptor {
                    label,
                    size: Extent3d {
                        width: size.width >> 1,
                        height: size.height >> 1,
                        depth_or_array_layers: size.depth_or_array_layers >> cascade,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D3,
                    format: Self::FORMAT,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                    view_formats: &[],
                })
            })
            .collect_vec();

        dbg!(cascades.iter().map(|x| x.size()).collect_vec());

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
            border_color: Some(SamplerBorderColor::Zero),
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
        .map(|((cascade_out, cascade_in), index)| {
            gpu.device().create_bind_group(&BindGroupDescriptor {
                label,
                layout: &Self::layout_cascade(gpu),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &radiance.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(
                            &cascade_out.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(
                            &cascade_in.create_view(&TextureViewDescriptor::default()),
                        ),
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
                    resource: BindingResource::TextureView(
                        &radiance.create_view(&TextureViewDescriptor::default()),
                    ),
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
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &radiance.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &cascades[0].create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            radiance,
            binding_read,
            binding_write,
            cascades,
            cascade_index_buffers,
            binding_cascades,
        }
    }

    pub fn size(&self) -> UVec3 {
        UVec3::new(
            self.radiance.width(),
            self.radiance.height(),
            self.radiance.depth_or_array_layers(),
        )
    }

    pub fn radiance(&self) -> &Texture {
        &self.radiance
    }

    pub fn cascades(&self) -> &[Texture] {
        &self.cascades
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
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
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
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

    pub fn layout_cascade(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
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
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
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
}

impl Drop for RadianceVolume {
    fn drop(&mut self) {
        self.radiance.destroy();

        for texture in &self.cascades {
            texture.destroy();
        }

        for buffer in &self.cascade_index_buffers {
            buffer.destroy();
        }
    }
}
