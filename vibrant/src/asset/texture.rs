use std::{any::type_name, marker::PhantomData};

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, CommandEncoder,
    Extent3d, FilterMode, ImageSubresourceRange, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderStages, StorageTextureAccess, Texture, TextureAspect, TextureDescriptor, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
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

pub struct Rgba8Unorm {}
pub struct R32Float {}
pub struct R32Uint {}

impl ScalarTextureFormat for R32Float {
    fn format() -> TextureFormat {
        TextureFormat::R32Float
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Float { filterable: true }
    }
}

impl ScalarTextureFormat for Rgba8Unorm {
    fn format() -> TextureFormat {
        TextureFormat::Rgba8Unorm
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Float { filterable: true }
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

pub type MipTexture3D<Format> = MipTexture<3, Format>;
pub type MipTexture2D<Format> = MipTexture<2, Format>;

pub struct MipTexture<const DIMENSION: u32, Format: ScalarTextureFormat> {
    texture: Texture,
    view: TextureView,
    sampler: Sampler,
    binding: BindGroup,
    binding_write: BindGroup,
    bindings_mipmap: Vec<BindGroup>,
    phantom: PhantomData<Format>,
}

impl<const DIMENSION: u32, Format: ScalarTextureFormat> MipTexture<DIMENSION, Format> {
    pub fn new(gpu: &Gpu, width: u32, height: u32, depth: u32, filter: FilterMode) -> Self {
        let label = Some(type_name::<Self>());

        let mip_level_count = width.min(height).ilog2();

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

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Format::format()),
            dimension: Some(Self::view_dimension()),
            ..Default::default()
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
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
            view,
            sampler,
            binding,
            binding_write,
            bindings_mipmap,
            phantom: PhantomData,
        }
    }

    pub fn resolution(&self) -> u32 {
        self.texture.width()
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
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

    pub fn binding_entries<'a>(&'a self, offset: u32) -> Vec<BindGroupEntry<'a>> {
        vec![
            BindGroupEntry {
                binding: offset + 0,
                resource: BindingResource::TextureView(&self.view),
            },
            BindGroupEntry {
                binding: offset + 1,
                resource: BindingResource::Sampler(&self.sampler),
            },
        ]
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_texture(
            &self.texture,
            &ImageSubresourceRange {
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
        );
    }

    pub fn layout_entries(offset: u32) -> Vec<BindGroupLayoutEntry> {
        vec![
            BindGroupLayoutEntry {
                binding: offset + 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: Format::sample_type(),
                    view_dimension: Self::view_dimension(),
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: offset + 1,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(Format::sampler_type()),
                count: None,
            },
        ]
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &Self::layout_entries(0),
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
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(Format::sampler_type()),
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

    fn view_dimension() -> TextureViewDimension {
        match DIMENSION {
            1 => wgpu::TextureViewDimension::D1,
            2 => wgpu::TextureViewDimension::D2,
            3 => wgpu::TextureViewDimension::D3,
            _ => panic!("Dimension should be between 1 and 3"),
        }
    }
}

impl<const DIMENSION: u32, Format: ScalarTextureFormat> Drop for MipTexture<DIMENSION, Format> {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
