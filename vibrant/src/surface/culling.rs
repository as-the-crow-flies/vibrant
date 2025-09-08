use std::{any::type_name, ops::Mul};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, Extent3d, FilterMode, ShaderStages,
    StorageTextureAccess, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

use crate::{
    asset::texture::{MipTexture3D, R32Float},
    gpu::Gpu,
};

pub struct CullingBuffer {
    offset: Buffer,
    offset_total: Buffer,
    index: Buffer,
    culling: MipTexture3D<R32Float>,
    erode: Texture,
    binding_read: BindGroup,
    binding_write: BindGroup,
}

impl CullingBuffer {
    pub fn new(gpu: &Gpu, resolution: u32) -> Self {
        let label = Some(type_name::<Self>());

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: resolution.pow(3).mul(4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let offset_total = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let index = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 1024 * 1024 * 1024,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let culling: MipTexture3D<R32Float> =
            MipTexture3D::new(gpu, resolution, resolution, resolution, FilterMode::Linear);

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &offset,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &offset_total,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &index,
                    offset: 0,
                    size: None,
                }),
            },
        ];

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries: &[entries.to_vec(), culling.binding_entries(3)].concat(),
        });

        let erode = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: resolution,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: TextureFormat::R32Float,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let erode_view = erode.create_view(&TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let erode_binding = BindGroupEntry {
            binding: 3,
            resource: BindingResource::TextureView(&erode_view),
        };

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries: &[entries.to_vec(), vec![erode_binding]].concat(),
        });

        Self {
            offset,
            offset_total,
            index,
            culling,
            erode,
            binding_read,
            binding_write,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.offset_total, 0, None);
    }

    pub fn count(&self) -> &Buffer {
        &self.offset_total
    }

    pub fn pyramid(&self) -> &MipTexture3D<R32Float> {
        &self.culling
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.binding_write
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    vec![
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    MipTexture3D::<R32Float>::layout_entries(3),
                ]
                .concat(),
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
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::R32Float,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for CullingBuffer {
    fn drop(&mut self) {
        self.offset.destroy();
        self.offset_total.destroy();
        self.index.destroy();
        self.erode.destroy();
    }
}
