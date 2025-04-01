use std::{any::type_name, ops::Add};

use bytemuck::bytes_of;
use glam::Mat4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferUsages, Extent3d, FilterMode, SamplerBindingType,
    SamplerDescriptor, ShaderStages, StorageTextureAccess, Texture, TextureDescriptor,
    TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub type ScalarTexture3D = ScalarTexture<3>;
pub type ScalarTexture2D = ScalarTexture<2>;

pub struct ScalarTexture<const DIMENSION: u32> {
    texture: Texture,
    transform: Buffer,
    transform_inverse: Buffer,
    binding: BindGroup,
    binding_write: BindGroup,
    bindings_mipmap: Vec<BindGroup>,
}

impl<const DIMENSION: u32> ScalarTexture<DIMENSION> {
    const TEXTURE_FORMAT: TextureFormat = TextureFormat::R8Unorm;

    pub fn new(gpu: &Gpu, width: u32, height: u32, depth: u32, transform: Mat4) -> Self {
        let label = Some(type_name::<Self>());

        let mip_level_count = match DIMENSION {
            1 => width,
            2 => width.min(height),
            3 => width.min(height).min(depth),
            _ => panic!("Texture Dimension should be between 1 and 3"),
        }
        .div_ceil(8)
        .add(1)
        .ilog2()
        .max(1);

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: depth,
            },
            mip_level_count,
            sample_count: 1,
            dimension: match DIMENSION {
                1 => wgpu::TextureDimension::D1,
                2 => wgpu::TextureDimension::D2,
                3 => wgpu::TextureDimension::D3,
                _ => panic!("Texture Dimension should be between 1 and 3"),
            },
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
            ..Default::default()
        });

        let transform_inverse = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&transform.inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
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
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform_inverse,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::TEXTURE_FORMAT),
                            mip_level_count: Some(1),
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform_inverse,
                        offset: 0,
                        size: None,
                    }),
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
                                    base_mip_level: level,
                                    mip_level_count: Some(1),
                                    ..Default::default()
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
            texture,
            transform,
            transform_inverse,
            binding,
            binding_write,
            bindings_mipmap,
        }
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    pub fn depth(&self) -> u32 {
        self.texture.depth_or_array_layers()
    }

    pub fn update(&self, gpu: &Gpu, transform: &Mat4) {
        gpu.queue()
            .write_buffer(&self.transform, 0, bytes_of(transform));
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
                            view_dimension: Self::view_dimension(),
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
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::all(),
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

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: Self::TEXTURE_FORMAT,
                            view_dimension: Self::view_dimension(),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::all(),
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
                            view_dimension: Self::view_dimension(),
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
                            view_dimension: Self::view_dimension(),
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

    fn view_dimension() -> TextureViewDimension {
        match DIMENSION {
            1 => wgpu::TextureViewDimension::D1,
            2 => wgpu::TextureViewDimension::D2,
            3 => wgpu::TextureViewDimension::D3,
            _ => panic!("Dimension should be between 1 and 3"),
        }
    }
}

impl<const DIMENSION: u32> Drop for ScalarTexture<DIMENSION> {
    fn drop(&mut self) {
        self.texture.destroy();
        self.transform.destroy();
        self.transform_inverse.destroy();
    }
}
