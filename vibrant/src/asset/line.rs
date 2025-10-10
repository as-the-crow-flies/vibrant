use std::{any::type_name, ops::Div};

use glam::{Mat4, Vec3};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, ShaderStages,
};

use crate::{file::LineFile, gpu::Gpu};

pub struct LineSet {
    buffer: LineBuffer,
    binding_read: BindGroup,
    binding_write: BindGroup,
    binding_raw: BindGroup,
}

impl LineSet {
    pub fn new(gpu: &Gpu, line: &LineFile) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = LineBuffer::new(gpu, line);

        let transform = Mat4::IDENTITY
            * Mat4::from_translation(line.bounds().min + 0.5 * line.bounds().scale())
            * Mat4::from_scale(Vec3::splat(line.bounds().scale().max_element()));

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform.inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu, true),
            entries: &buffer.entries(&buffer.indices, &buffer.vertices),
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu, false),
            entries: &buffer.entries(&buffer.indices, &buffer.vertices),
        });

        let binding_raw = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_raw(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.indices_raw.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffer.vertices_raw.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: transform.as_entire_binding(),
                },
            ],
        });

        Self {
            buffer,
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

    pub fn len(&self) -> u32 {
        self.buffer.indices.size().div(4) as u32
    }

    pub fn indices(&self) -> &Buffer {
        &self.buffer.indices
    }

    pub fn vertices(&self) -> &Buffer {
        &self.buffer.vertices
    }

    pub fn count(&self) -> &Buffer {
        &self.buffer.count
    }

    pub fn cull(&self) -> &Buffer {
        &self.buffer.cull_vertex
    }

    pub fn binding_raw(&self) -> &BindGroup {
        &self.binding_raw
    }

    pub fn clear_total_count(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.buffer.total_count, 0, None);
    }

    pub fn clear_count(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.buffer.count, 0, None);
    }

    pub fn clear_cull(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.buffer.cull_vertex, 0, None);
    }

    pub fn layout_raw(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Indices Raw
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
                    // Vertices Raw
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
                    // Transform
                    BindGroupLayoutEntry {
                        binding: 2,
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
        let visibility = if read_only {
            ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT
        } else {
            ShaderStages::COMPUTE | ShaderStages::FRAGMENT
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Indices
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Vertices
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Total Count
                    BindGroupLayoutEntry {
                        binding: 2,
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
                        binding: 3,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Cull Vertex
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Line Counts
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Line Offsets
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}

struct LineBuffer {
    vertices_raw: Buffer,
    indices_raw: Buffer,
    vertices: Buffer,
    indices: Buffer,
    total_count: Buffer,
    count: Buffer,
    cull_vertex: Buffer,
    line_counts: Buffer,
    line_offsets: Buffer,
}

impl LineBuffer {
    fn new(gpu: &Gpu, line: &LineFile) -> Self {
        let label = Some(type_name::<Self>());

        let vertices_raw = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&line.vertices()),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let indices_raw = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&line.indices()),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let vertices = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: vertices_raw.size(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&line.indices()),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });

        let total_count = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&(line.indices().len() as u32)),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });

        let count = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let cull_vertex = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (line.indices().len() * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let line_counts = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&line.line_counts()),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let line_offsets = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&line.line_offsets()),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        Self {
            vertices_raw,
            indices_raw,
            vertices,
            indices,
            total_count,
            count,
            cull_vertex,
            line_counts,
            line_offsets,
        }
    }

    fn entries<'a>(&'a self, indices: &'a Buffer, vertices: &'a Buffer) -> Vec<BindGroupEntry<'a>> {
        vec![
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: indices,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: vertices,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &self.total_count,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &self.count,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &self.cull_vertex,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 5,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &self.line_counts,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 6,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &self.line_offsets,
                    offset: 0,
                    size: None,
                }),
            },
        ]
    }
}

impl Drop for LineBuffer {
    fn drop(&mut self) {
        self.vertices_raw.destroy();
        self.vertices.destroy();
        self.indices.destroy();
        self.total_count.destroy();
        self.count.destroy();
        self.cull_vertex.destroy();
        self.line_counts.destroy();
        self.line_offsets.destroy();
    }
}
