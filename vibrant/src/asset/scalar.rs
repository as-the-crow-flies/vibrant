use std::{any::type_name, marker::PhantomData};

use bytemuck::bytes_of;
use glam::{Mat4, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferUsages, Extent3d, FilterMode, SamplerBindingType,
    SamplerDescriptor, ShaderStages, StorageTextureAccess, Texture, TextureDescriptor,
    TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub trait ScalarTextureFormat {
    fn format() -> TextureFormat;
    fn sample_type() -> TextureSampleType;
    fn sampler_type() -> SamplerBindingType {
        match Self::sample_type() {
            TextureSampleType::Float { filterable: true } => SamplerBindingType::Filtering,
            _ => SamplerBindingType::NonFiltering,
        }
    }
}

pub struct R8Unorm {}
pub struct R8Uint {}
pub struct R16Uint {}
pub struct R32Uint {}

impl ScalarTextureFormat for R8Unorm {
    fn format() -> TextureFormat {
        TextureFormat::R8Unorm
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Float { filterable: true }
    }
}

impl ScalarTextureFormat for R8Uint {
    fn format() -> TextureFormat {
        TextureFormat::R8Uint
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Uint
    }
}

impl ScalarTextureFormat for R16Uint {
    fn format() -> TextureFormat {
        TextureFormat::R16Uint
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Uint
    }
}

impl ScalarTextureFormat for R32Uint {
    fn format() -> TextureFormat {
        TextureFormat::R32Uint
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Uint
    }
}

pub type ScalarTexture3D<Format> = ScalarTexture<3, Format>;
pub type ScalarTexture2D<Format> = ScalarTexture<2, Format>;

pub struct ScalarTexture<const DIMENSION: u32, Format: ScalarTextureFormat> {
    texture: Texture,
    transform: Buffer,
    transform_inverse: Buffer,
    binding: BindGroup,
    binding_write: BindGroup,
    bindings_mipmap: Vec<BindGroup>,
    phantom: PhantomData<Format>,
}

impl<const DIMENSION: u32, Format: ScalarTextureFormat> ScalarTexture<DIMENSION, Format> {
    pub fn new(gpu: &Gpu, volume: u32, filter: FilterMode) -> Self {
        let label = Some(type_name::<Self>());

        let mip_level_count = volume.ilog2();

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: volume,
                height: volume,
                depth_or_array_layers: volume,
            },
            mip_level_count,
            sample_count: 1,
            dimension: match DIMENSION {
                1 => wgpu::TextureDimension::D1,
                2 => wgpu::TextureDimension::D2,
                3 => wgpu::TextureDimension::D3,
                _ => panic!("Texture Dimension should be between 1 and 3"),
            },
            format: Format::format(),
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
            view_formats: &[Format::format()],
        });

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: filter,
            ..Default::default()
        });

        let scale = volume as f32;
        let transform = Mat4::from_translation(Vec3::new(scale / 2.0, scale / 2.0, scale / 2.0))
            * Mat4::from_scale(Vec3::new(scale, scale, scale));

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
                            format: Some(Format::format()),
                            dimension: Some(Self::view_dimension()),
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
                            format: Some(Format::format()),
                            dimension: Some(Self::view_dimension()),
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
                                    format: Some(Format::format()),
                                    dimension: Some(Self::view_dimension()),
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
                                    format: Some(Format::format()),
                                    dimension: Some(Self::view_dimension()),
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
            phantom: PhantomData,
        }
    }

    pub fn size(&self) -> u32 {
        self.texture.width()
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
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
                            sample_type: Format::sample_type(),
                            view_dimension: Self::view_dimension(),
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Sampler(Format::sampler_type()),
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
                            format: Format::format(),
                            view_dimension: Self::view_dimension(),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Sampler(Format::sampler_type()),
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
                            sample_type: Format::sample_type(),
                            view_dimension: Self::view_dimension(),
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(Format::sampler_type()),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Format::format(),
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

impl<const DIMENSION: u32, Format: ScalarTextureFormat> Drop for ScalarTexture<DIMENSION, Format> {
    fn drop(&mut self) {
        self.texture.destroy();
        self.transform.destroy();
        self.transform_inverse.destroy();
    }
}
