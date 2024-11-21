use std::{any::type_name, f32::consts::PI};

use bytemuck::bytes_of;
use glam::{Mat4, Quat, Vec3};

use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, CommandEncoder, ShaderStages,
};

use crate::{gpu::Gpu, loader};

pub struct Tractogram {
    vertices: Buffer,
    indices: Buffer,
    caps: Buffer,
    binding: BindGroup,
    binding_full: BindGroup,
    binding_full_write: BindGroup,
    count: TractogramCount,
}

impl Tractogram {
    pub fn vertices(&self) -> &Buffer {
        &self.vertices
    }

    pub fn caps(&self) -> &Buffer {
        &self.caps
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn binding_full(&self) -> &BindGroup {
        &self.binding_full
    }

    pub fn binding_full_write(&self) -> &BindGroup {
        &self.binding_full_write
    }

    pub fn vertex_count(&self) -> u32 {
        (self.vertices.size() / 12) as u32
    }

    pub fn cap_count(&self) -> u32 {
        (self.caps.size() / 24) as u32
    }

    pub fn count(&self) -> &TractogramCount {
        &self.count
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
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

    pub fn layout_full(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::all(),
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

    pub fn layout_full_write(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
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
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
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

    pub fn new(gpu: &Gpu, tractogram: &loader::Tck) -> Self {
        let label = Some(type_name::<Self>());

        let indices = tractogram
            .vertices()
            .iter()
            .enumerate()
            .filter_map(|(index, &vertex)| vertex.is_finite().then_some(index as u32))
            .collect_vec();

        let vertices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(tractogram.vertices()),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let caps = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(tractogram.caps()),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let indices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::STORAGE,
        });

        let scale = tractogram.bounds().scale();
        let transform = Mat4::from_scale_rotation_translation(
            Vec3::new(scale, scale, scale),
            Quat::from_rotation_x(0.5 * PI),
            Vec3::ZERO,
        );

        let world_to_tractogram = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
        });

        let tractogram_to_world = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform.inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &tractogram_to_world,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &world_to_tractogram,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let binding_full = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_full(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &tractogram_to_world,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &world_to_tractogram,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &vertices,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &indices,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let binding_full_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_full_write(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &tractogram_to_world,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &world_to_tractogram,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &vertices,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &indices,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            count: TractogramCount::new(gpu, (indices.size() / 4) as u32),
            vertices,
            indices,
            caps,
            binding,
            binding_full,
            binding_full_write,
        }
    }
}

impl Drop for Tractogram {
    fn drop(&mut self) {
        self.vertices.destroy();
        self.indices.destroy();
    }
}

pub struct TractogramCount {
    buffer: Buffer,
    binding: BindGroup,
}

impl TractogramCount {
    pub fn new(gpu: &Gpu, count: u32) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&count),
            usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self { buffer, binding }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.buffer, 0, None);
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}
