use std::any::type_name;

use bytemuck::bytes_of;
use glam::{Mat4, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, ShaderStages,
};

use crate::{asset::scalar::ScalarTexture3D, gpu::Gpu};

pub struct Density {
    texture: ScalarTexture3D,
    buffer: Buffer,
    transform: Buffer,
    binding: BindGroup,
}

impl Density {
    pub fn new(gpu: &Gpu, volume: u32) -> Self {
        let label = Some(type_name::<Self>());

        let scale = volume as f32;

        let transform = Mat4::from_translation(Vec3::new(scale / 2.0, scale / 2.0, scale / 2.0))
            * Mat4::from_scale(Vec3::new(scale, scale, scale));

        let texture = ScalarTexture3D::new(
            gpu,
            volume,
            volume,
            volume,
            transform,
            wgpu::FilterMode::Linear,
        );

        let n_voxels = volume * volume * volume;

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (n_voxels * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            texture,
            buffer,
            transform,
            binding,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.buffer, 0, None);
    }

    pub fn texture(&self) -> &ScalarTexture3D {
        &self.texture
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
                        visibility: ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for Density {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.transform.destroy();
    }
}
