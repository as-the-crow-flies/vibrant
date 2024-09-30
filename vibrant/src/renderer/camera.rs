use std::any::type_name;

use bytemuck::bytes_of;
use glam::Mat4;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ShaderStages,
};

use crate::gpu::Gpu;

pub struct Camera {
    pub layout: BindGroupLayout,
    pub binding: BindGroup,
    pub buffer: Buffer,
}

impl Camera {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layout = gpu
            .device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            layout,
            binding,
            buffer,
        }
    }

    pub fn update(&self, gpu: &Gpu, mvp: Mat4) {
        gpu.queue().write_buffer(&self.buffer, 0, bytes_of(&mvp));
    }
}
