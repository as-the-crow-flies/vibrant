use std::any::type_name;

use glam::{Mat4, Vec3};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, ShaderStages,
};

use crate::{file, gpu::Gpu};

pub struct LineSet {
    vertices_raw: Buffer,
    vertices: Buffer,
    indices: Buffer,
    count: Buffer,
    binding_read: BindGroup,
    binding_write: BindGroup,
    binding_raw: BindGroup,
}

impl LineSet {
    pub fn new(gpu: &Gpu, line: &file::LineFile) -> Self {
        let label = Some(type_name::<Self>());

        let vertices_raw = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&line.vertices()),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let vertices = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: vertices_raw.size(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&line.indices()),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let count = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transform = Mat4::IDENTITY
            * Mat4::from_translation(line.bounds().min)
            * Mat4::from_scale(Vec3::splat(line.bounds().scale().max_element()));

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform.inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &indices,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &vertices,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &count,
                    offset: 0,
                    size: None,
                }),
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

        let binding_raw = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_raw(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &vertices_raw,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            vertices_raw,
            vertices,
            indices,
            count,
            binding_read,
            binding_write,
            binding_raw,
        }
    }

    pub fn binding(&self, read_only: bool) -> &BindGroup {
        if read_only {
            &self.binding_read
        } else {
            &self.binding_write
        }
    }

    pub fn binding_raw(&self) -> &BindGroup {
        &self.binding_raw
    }

    pub fn clear_count(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.count, 0, None);
    }

    pub fn layout_raw(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Vertices Raw
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
                    // Transform
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
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

    pub fn layout(gpu: &Gpu, read_only: bool) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Indices
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
                    // Vertices
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Count
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for LineSet {
    fn drop(&mut self) {
        self.vertices_raw.destroy();
        self.vertices.destroy();
        self.indices.destroy();
        self.count.destroy();
    }
}
