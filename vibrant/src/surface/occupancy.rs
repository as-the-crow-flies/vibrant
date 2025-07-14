use std::{any::type_name, ops::Mul};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, FilterMode, ShaderStages,
};

use crate::{
    asset::scalar::{R32Float, ScalarTexture3D},
    gpu::Gpu,
};

pub struct Occupancy {
    offset: Buffer,
    count: Buffer,
    index: Buffer,
    bin: Buffer,
    threshold: Buffer,
    occupancy: ScalarTexture3D<R32Float>,
    binding_read: BindGroup,
    binding_write: BindGroup,
}

impl Occupancy {
    pub fn new(gpu: &Gpu, volume: u32, memory: u32) -> Self {
        let label = Some(type_name::<Self>());

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: volume.pow(3).mul(4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let count = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let index = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: memory.mul(1024 * 1024) as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let bin = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 1024,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let threshold = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let occupancy: ScalarTexture3D<R32Float> =
            ScalarTexture3D::new(gpu, volume, FilterMode::Linear);

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
                    buffer: &count,
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
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &bin,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &threshold,
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

        Self {
            offset,
            count,
            index,
            bin,
            threshold,
            occupancy,
            binding_read,
            binding_write,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.count, 0, None);
        cmd.clear_buffer(&self.bin, 0, None);
    }

    pub fn count(&self) -> &Buffer {
        &self.count
    }

    pub fn bin(&self) -> &Buffer {
        &self.bin
    }

    pub fn threshold(&self) -> &Buffer {
        &self.threshold
    }

    pub fn texture(&self) -> &ScalarTexture3D<R32Float> {
        &self.occupancy
    }

    pub fn binding(&self, read_only: bool) -> &BindGroup {
        if read_only {
            &self.binding_read
        } else {
            &self.binding_write
        }
    }

    pub fn layout(gpu: &Gpu, read_only: bool) -> BindGroupLayout {
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
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
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
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
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

impl Drop for Occupancy {
    fn drop(&mut self) {
        self.offset.destroy();
        self.count.destroy();
        self.index.destroy();
        self.bin.destroy();
        self.threshold.destroy();
    }
}
