use std::{any::type_name, marker::PhantomData};

use wgpu::*;

use crate::gpu::Gpu;

pub trait MipTextureFormat {
    fn format() -> TextureFormat;
    fn sample_type() -> TextureSampleType;
    fn sampler_type() -> SamplerBindingType {
        match Self::sample_type() {
            TextureSampleType::Float { filterable: true } => SamplerBindingType::Filtering,
            _ => SamplerBindingType::NonFiltering,
        }
    }
}

pub struct Rgba16Float {}
pub struct Rgba8Unorm {}
pub struct R32Float {}
pub struct R32Uint {}

impl MipTextureFormat for R32Float {
    fn format() -> TextureFormat {
        TextureFormat::R32Float
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Float { filterable: true }
    }
}

impl MipTextureFormat for Rgba16Float {
    fn format() -> TextureFormat {
        TextureFormat::Rgba16Float
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Float { filterable: true }
    }
}

impl MipTextureFormat for Rgba8Unorm {
    fn format() -> TextureFormat {
        TextureFormat::Rgba8Unorm
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Float { filterable: true }
    }
}

impl MipTextureFormat for R32Uint {
    fn format() -> TextureFormat {
        TextureFormat::R32Uint
    }

    fn sample_type() -> TextureSampleType {
        TextureSampleType::Uint
    }
}

pub type MipTexture3D<Format> = MipTexture<3, Format>;
pub type MipTexture2D<Format> = MipTexture<2, Format>;

pub struct MipTexture<const DIMENSION: u32, Format: MipTextureFormat> {
    texture: Texture,
    view: TextureView,
    sampler: Sampler,
    binding: BindGroup,
    binding_write: BindGroup,
    bindings_mipmap: Vec<BindGroup>,
    phantom: PhantomData<Format>,
}

impl<const DIMENSION: u32, Format: MipTextureFormat> MipTexture<DIMENSION, Format> {
    pub fn new(
        gpu: &Gpu,
        width: u32,
        height: u32,
        depth: u32,
        filter: FilterMode,
        address: AddressMode,
    ) -> Self {
        let label = Some(type_name::<Self>());

        let mip_level_count = width.min(height).ilog2().max(1);

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
            address_mode_u: address,
            address_mode_v: address,
            address_mode_w: address,
            border_color: None,
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

    pub fn size(&self) -> Extent3d {
        self.texture.size()
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

    pub fn copy_from(&self, cmd: &mut CommandEncoder, source: &Texture) {
        cmd.copy_texture_to_texture(
            TexelCopyTextureInfo {
                texture: source,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            self.size(),
        );
    }

    pub fn mipmap(&self, cmd: &mut CommandEncoder, pipeline: &ComputePipeline) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(pipeline);
        for (level, binding) in self.bindings_mipmap().iter().enumerate() {
            let size = self
                .size()
                .mip_level_size(level as u32, TextureDimension::D2);
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(
                size.width.div_ceil(4),
                size.height.div_ceil(4),
                size.depth_or_array_layers.div_ceil(4),
            );
        }
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

impl<const DIMENSION: u32, Format: MipTextureFormat> Drop for MipTexture<DIMENSION, Format> {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
