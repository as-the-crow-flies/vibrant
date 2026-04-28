use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use wgpu::{Buffer, BufferDescriptor, BufferUsages};

use crate::gpu::Gpu;

pub struct Buffered<T: Pod + Zeroable> {
    data: T,
    buffer: Buffer,
}

impl<T: Pod + Zeroable> Buffered<T> {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            data: T::zeroed(),
            buffer: gpu.device().create_buffer(&BufferDescriptor {
                label: Some(type_name::<Self>()),
                size: size_of::<T>() as u64,
                usage: BufferUsages::UNIFORM,
                mapped_at_creation: false,
            }),
        }
    }

    pub fn data(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn update(&self, gpu: &Gpu) {
        gpu.queue()
            .write_buffer(&self.buffer, 0, bytes_of(&self.data));
    }
}
