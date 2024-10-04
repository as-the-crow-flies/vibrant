use std::any::type_name;

use wgpu::{Buffer, BufferDescriptor, BufferUsages};

use crate::gpu::Gpu;

pub struct Density {
    buffer: Buffer,
}

impl Density {
    pub fn new(gpu: &Gpu, size: u32) -> Self {
        let label = Some(type_name::<Self>());

        Self {
            buffer: gpu.device().create_buffer(&BufferDescriptor {
                label,
                size: (size * size * size * 4) as u64,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        }
    }
}
