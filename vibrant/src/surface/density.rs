use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, ShaderStages,
};

use crate::{
    asset::scalar::{R8Uint, R8Unorm, ScalarTexture3D},
    gpu::Gpu,
};

pub struct Density {
    density: ScalarTexture3D<R8Unorm>,
    count: ScalarTexture3D<R8Uint>,
    buffer: Buffer,
    binding: BindGroup,
}

impl Density {
    pub fn new(gpu: &Gpu, volume: u32) -> Self {
        let label = Some(type_name::<Self>());

        let density = ScalarTexture3D::<R8Unorm>::new(gpu, volume, wgpu::FilterMode::Linear);
        let count = ScalarTexture3D::<R8Uint>::new(gpu, volume, wgpu::FilterMode::Nearest);

        let n_voxels = volume * volume * volume;

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (n_voxels * 4) as u64,
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
            density,
            count,
            buffer,
            binding,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.buffer, 0, None);
    }

    pub fn resolution(&self) -> u32 {
        self.density.width()
    }

    pub fn volume(&self) -> &ScalarTexture3D<R8Unorm> {
        &self.density
    }

    pub fn count(&self) -> &ScalarTexture3D<R8Uint> {
        &self.count
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

impl Drop for Density {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}
