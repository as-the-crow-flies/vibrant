use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, ShaderStages,
};

use crate::{constants::DENSITY_SIZE, gpu::Gpu};

use super::scalar::ScalarTexture;

pub struct Density {
    buffer: Buffer,
    binding: BindGroup,
    texture: ScalarTexture,
}

impl Density {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (DENSITY_SIZE * DENSITY_SIZE * DENSITY_SIZE * 4) as u64,
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
            buffer,
            binding,
            texture: ScalarTexture::new(gpu, DENSITY_SIZE, DENSITY_SIZE, DENSITY_SIZE),
        }
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

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn texture(&self) -> &ScalarTexture {
        &self.texture
    }
}

impl Drop for Density {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}
