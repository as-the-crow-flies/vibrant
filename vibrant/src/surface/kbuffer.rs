use std::any::type_name;

use wgpu::{
    wgt::BufferDescriptor, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType,
    BufferUsages, ShaderStages,
};

use crate::gpu::Gpu;

pub struct KBuffer {
    kbuffer: Buffer,
    lock: Buffer,
    binding: BindGroup,
}

impl KBuffer {
    pub fn new(gpu: &Gpu, width: u32, height: u32, k: u32) -> Self {
        let label = Some(type_name::<Self>());

        let pixel_count = width * height;

        let kbuffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (pixel_count * k * 8) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let lock = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (pixel_count * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: kbuffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: lock.as_entire_binding(),
                },
            ],
        });

        Self {
            kbuffer,
            lock,
            binding,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
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

    pub fn clear(&self, cmd: &mut wgpu::CommandEncoder) {
        cmd.clear_buffer(&self.kbuffer, 0, None);
        cmd.clear_buffer(&self.lock, 0, None);
    }
}

impl Drop for KBuffer {
    fn drop(&mut self) {
        self.kbuffer.destroy();
    }
}
