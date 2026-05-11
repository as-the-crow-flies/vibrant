use std::{any::type_name, f32::consts::PI};

use bytemuck::{bytes_of, Pod, Zeroable};
use glam::Vec4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, ShaderStages,
};

use crate::gpu::Gpu;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
pub struct CropSettings {
    pub spherical: Vec4,
    pub min: Vec4,
    pub max: Vec4,
}

#[derive(Debug)]
pub struct CropBuffer {
    settings: CropSettings,
    buffer: Buffer,
    binding: BindGroup,
}

impl CropBuffer {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let settings = CropSettings {
            spherical: Vec4::new(0.0, 0.5 * PI, 0.0, 0.01),
            min: Vec4::NEG_ONE,
            max: Vec4::ONE,
        };

        let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&settings),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            settings,
            buffer,
            binding,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn settings_mut(&mut self) -> &mut CropSettings {
        &mut self.settings
    }

    pub fn update_settings(&self, gpu: &Gpu) {
        gpu.queue()
            .write_buffer(&self.buffer, 0, bytes_of(&self.settings));
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}
