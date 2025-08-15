use std::{any::type_name, ops::Div};

use glam::{Mat4, Vec3};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, ShaderStages,
};

use crate::{file::LineFile, gpu::Gpu, sort::KeyValuePair};

pub struct LineSet {
    vertices_raw: Buffer,
    vertices: Buffer,
    indices: Buffer,
    count: Buffer,
    cull: Buffer,
    binding_read: BindGroup,
    binding_write: BindGroup,
    binding_raw: BindGroup,
    sorted: SortedLineSet,
}

impl LineSet {
    pub fn new(gpu: &Gpu, line: &LineFile) -> Self {
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
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let count = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let cull = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (line.indices().len() * 4) as u64,
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
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &cull,
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

        let sorted =
            SortedLineSet::new(gpu, &vertices, &count, &cull, indices.size().div(4) as u32);

        Self {
            vertices_raw,
            vertices,
            indices,
            count,
            cull,
            binding_read,
            binding_write,
            binding_raw,
            sorted,
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
        self.indices.size().div(4) as u32
    }

    pub fn indices(&self) -> &Buffer {
        &self.indices
    }

    pub fn vertices(&self) -> &Buffer {
        &self.vertices
    }

    pub fn count(&self) -> &Buffer {
        &self.count
    }

    pub fn cull(&self) -> &Buffer {
        &self.cull
    }

    pub fn sorted(&self) -> &SortedLineSet {
        &self.sorted
    }

    pub fn binding_raw(&self) -> &BindGroup {
        &self.binding_raw
    }

    pub fn clear_count(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.count, 0, None);
    }

    pub fn clear_cull(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.cull, 0, None);
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
                            ty: BufferBindingType::Storage { read_only: true },
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
                    // Cull
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
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

pub struct SortedLineSet {
    ping: KeyValuePair,
    pong: KeyValuePair,
    binding_read: BindGroup,
    binding_write: BindGroup,
    linear: Buffer,
}

impl SortedLineSet {
    pub fn new(gpu: &Gpu, vertices: &Buffer, count: &Buffer, cull: &Buffer, len: u32) -> Self {
        let ping = KeyValuePair::new(gpu, len);
        let pong = KeyValuePair::new(gpu, len);

        let linear: Vec<u32> = (0..=len).collect();

        let linear = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("SortedLineSet::Linear"),
            contents: bytemuck::cast_slice(&linear),
            usage: BufferUsages::COPY_SRC,
        });

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: ping.key().as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: vertices.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: count.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: cull.as_entire_binding(),
            },
        ];

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some("LineSort::Line"),
            layout: &LineSet::layout(gpu, true),
            entries,
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some("LineSort::Line"),
            layout: &LineSet::layout(gpu, false),
            entries,
        });

        Self {
            ping,
            pong,
            binding_read,
            binding_write,
            linear,
        }
    }

    pub fn binding(&self, read_only: bool) -> &BindGroup {
        if read_only {
            &self.binding_read
        } else {
            &self.binding_write
        }
    }

    pub fn ping(&self) -> &KeyValuePair {
        &self.ping
    }

    pub fn pong(&self) -> &KeyValuePair {
        &self.pong
    }

    pub fn linear(&self) -> &Buffer {
        &self.linear
    }
}
