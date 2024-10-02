use std::any::type_name;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::{gpu::Gpu, loader};

pub struct Tractogram {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}

impl Tractogram {
    pub fn vertices(&self) -> &wgpu::Buffer {
        &self.vertices
    }

    pub fn indices(&self) -> &wgpu::Buffer {
        &self.indices
    }

    pub fn count(&self) -> u32 {
        (self.indices.size() / 4) as u32
    }

    pub fn vertex_buffer_layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: 12,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        }
    }

    pub fn new(gpu: &Gpu, tractogram: &loader::Tractogram) -> Self {
        Self {
            vertices: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label: Some(type_name::<Self>()),
                contents: bytemuck::cast_slice(&tractogram.vertices),
                usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
            }),
            indices: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label: Some(type_name::<Self>()),
                contents: bytemuck::cast_slice(&tractogram.indices),
                usage: BufferUsages::INDEX | BufferUsages::STORAGE,
            }),
        }
    }
}

impl Drop for Tractogram {
    fn drop(&mut self) {
        self.vertices.destroy();
        self.indices.destroy();
    }
}
