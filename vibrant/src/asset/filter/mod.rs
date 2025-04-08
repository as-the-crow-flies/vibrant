use std::any::type_name;

use bytemuck::{bytes_of, cast_slice};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, CommandEncoder, ComputePass, ComputePipeline, ShaderStages,
};

use crate::gpu::Gpu;

pub struct Filter {
    indices: Buffer,
    count: Buffer,
    workgroup_count: Buffer,
    workgroup_count_32: Buffer,
    read: BindGroup,
    write: BindGroup,
    copy: ComputePipeline,
}

impl Filter {
    pub const WORKGROUP_SIZE: usize = 1024;

    pub fn new(gpu: &Gpu, indices: &[u32]) -> Self {
        let label = Some(type_name::<Self>());

        let count = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&indices.len()),
            usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let workgroup_count = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&(indices.len().div_ceil(Self::WORKGROUP_SIZE) as u32)),
            usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let workgroup_count_32 = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&(indices.len().div_ceil(Self::WORKGROUP_SIZE * 32) as u32)),
            usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });

        let indices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: cast_slice(&indices),
            usage: BufferUsages::STORAGE,
        });

        let read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries: &[
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
                        buffer: &count,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries: &[
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
                        buffer: &count,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &workgroup_count,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &workgroup_count_32,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let copy = gpu.compute(
            "Filter::Copy",
            &gpu.pipeline_layout(&[&Filter::layout_write(gpu)]),
            &gpu.shader(include_str!("copy.wgsl")),
            "compute",
        );

        Self {
            count,
            workgroup_count,
            workgroup_count_32,
            indices,
            read,
            write,
            copy,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.count, 0, None);
    }

    pub fn copy(&self, pass: &mut ComputePass) {
        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, self.binding_write(), &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    pub fn copy_workgroup_count(&self, cmd: &mut CommandEncoder, buffer: &Buffer) {
        cmd.copy_buffer_to_buffer(self.workgroup_count(), 0, &buffer, 0, 4);
    }

    pub fn count(&self) -> &Buffer {
        &self.count
    }

    pub fn workgroup_count(&self) -> &Buffer {
        &self.workgroup_count
    }

    pub fn workgroup_count_32(&self) -> &Buffer {
        &self.workgroup_count_32
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.read
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.write
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
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

impl Drop for Filter {
    fn drop(&mut self) {
        self.indices.destroy();
        self.count.destroy();
        self.workgroup_count.destroy();
        self.workgroup_count_32.destroy();
    }
}
