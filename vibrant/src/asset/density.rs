use std::any::type_name;

use glam::{Mat4, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferDescriptor, BufferUsages, Extent3d, FilterMode,
    SamplerBindingType, SamplerDescriptor, ShaderStages, StorageTextureAccess, Texture,
    TextureAspect, TextureDescriptor, TextureFormat, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct Density {
    buffer: Buffer,
    texture: Texture,
    world_to_volume: Buffer,
    volume_to_world: Buffer,
    binding_compute: BindGroup,
    binding_render: BindGroup,
    binding_copy: BindGroup,
    bindings_mipmap: Vec<BindGroup>,
    size: u32,
}

impl Density {
    const TEXTURE_FORMAT: TextureFormat = TextureFormat::R32Float;

    pub fn binding_compute(&self) -> &BindGroup {
        &self.binding_compute
    }

    pub fn binding_render(&self) -> &BindGroup {
        &self.binding_render
    }

    pub fn binding_copy(&self) -> &BindGroup {
        &self.binding_copy
    }

    pub fn bindings_mipmap(&self) -> &[BindGroup] {
        &self.bindings_mipmap
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn new(gpu: &Gpu, exponent: u32) -> Self {
        let label = Some(type_name::<Self>());
        let size = 2u32.pow(exponent);
        let mip_level_count = exponent - 2;

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (size * size * size * 4) as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

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
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
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

        let scale = size as f32;
        let transform = Mat4::from_translation(Vec3::new(scale / 2.0, scale / 2.0, scale / 2.0))
            * Mat4::from_scale(Vec3::new(scale, scale, scale));

        let world_to_volume = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
        });

        let volume_to_world = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform.inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let binding_compute = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_compute(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &world_to_volume,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &volume_to_world,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let binding_render = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_render(gpu),
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

        let binding_copy = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_copy(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
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
                },
            ],
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
            buffer,
            texture,
            world_to_volume,
            volume_to_world,
            binding_compute,
            binding_render,
            binding_copy,
            bindings_mipmap,
            size,
        }
    }

    pub fn layout_compute(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
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

    pub fn layout_copy(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
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

    pub fn layout_render(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for Density {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.texture.destroy();
    }
}
