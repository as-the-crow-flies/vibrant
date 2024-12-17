use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, ShaderStages,
};

use crate::{asset::scalar::ScalarTexture, gpu::Gpu};

pub struct Occlusion {
    texture: ScalarTexture,
    buffer: Buffer,
    binding: BindGroup,
}

impl Occlusion {
    pub fn new(gpu: &Gpu, width: u32, height: u32, depth: u32) -> Self {
        let label = Some(type_name::<Self>());

        let texture = ScalarTexture::new(gpu, width, height, depth);

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (width * height * depth * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
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

        Self {
            texture,
            buffer,
            binding,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.buffer, 0, None);
    }

    pub fn texture(&self) -> &ScalarTexture {
        &self.texture
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
                    ty: wgpu::BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}

impl Drop for Occlusion {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}
