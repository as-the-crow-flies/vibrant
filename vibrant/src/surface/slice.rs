use std::any::type_name;

use glam::Mat4;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, Extent3d, FilterMode, ShaderStages,
    StorageTextureAccess, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

use crate::{asset::scalar::ScalarTexture2D, gpu::Gpu};

pub struct SliceBuffer {
    color: Buffer,
    bins: Texture,
    loz: Texture,
    absorbance: Texture,
    hiz: ScalarTexture2D,
    binding_read: BindGroup,
    binding_write: BindGroup,
}

impl SliceBuffer {
    pub fn new(gpu: &Gpu, width: u32, height: u32, tile: u32, layers: u32) -> Self {
        let label = Some(type_name::<Self>());

        let color = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (width * height * layers * 2 * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let width = width.div_ceil(tile);
        let height = height.div_ceil(tile);

        let bins = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 256,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: TextureFormat::R8Uint,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let bins_view = bins.create_view(&TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let loz = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let loz_view = loz.create_view(&TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let absorbance = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let absorbance_view = absorbance.create_view(&TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let hiz = ScalarTexture2D::new(gpu, width, height, 1, Mat4::IDENTITY, FilterMode::Nearest);

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &color,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&bins_view),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&loz_view),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(&absorbance_view),
            },
        ];

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu, true),
            entries,
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu, false),
            entries,
        });

        Self {
            color,
            bins,
            loz,
            absorbance,
            hiz,
            binding_read,
            binding_write,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.color, 0, None);
    }

    pub fn hiz(&self) -> &ScalarTexture2D {
        &self.hiz
    }

    pub fn binding(&self, read_only: bool) -> &BindGroup {
        if read_only {
            &self.binding_read
        } else {
            &self.binding_write
        }
    }

    pub fn layout(gpu: &Gpu, read_only: bool) -> BindGroupLayout {
        let access = if read_only {
            StorageTextureAccess::ReadOnly
        } else {
            StorageTextureAccess::ReadWrite
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::StorageTexture {
                            access,
                            format: TextureFormat::R8Uint,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::StorageTexture {
                            access,
                            format: TextureFormat::R8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::StorageTexture {
                            access,
                            format: TextureFormat::R8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for SliceBuffer {
    fn drop(&mut self) {
        self.color.destroy();
        self.bins.destroy();
        self.loz.destroy();
        self.absorbance.destroy();
    }
}
