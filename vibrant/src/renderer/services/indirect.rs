use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, ShaderStages,
};

use crate::gpu::Gpu;

pub struct ComputeIndirect {
    binding: BindGroup,
    indirect: Buffer,
    clear: Buffer,
}

impl ComputeIndirect {
    pub fn new(gpu: &Gpu, clear: [u32; 3]) -> Self {
        let label = Some(type_name::<Self>());

        let indirect = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 12,
            usage: BufferUsages::INDIRECT | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &indirect,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            binding,
            indirect,
            clear: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytes_of(&clear),
                usage: BufferUsages::COPY_SRC,
            }),
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn indirect(&self) -> &Buffer {
        &self.indirect
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.copy_buffer_to_buffer(&self.clear, 0, &self.indirect, 0, 12);
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
