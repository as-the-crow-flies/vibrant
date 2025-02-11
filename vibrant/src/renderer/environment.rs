use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ShaderStages,
};

use crate::{controller::Controller, gpu::Gpu};

pub struct Environment {
    binding: BindGroup,
    buffer: Buffer,
}

impl Environment {
    pub fn wgsl() -> String {
        "
        struct Settings {
            streamline_radius: f32,
            direct_light: f32,
            culling_threshold: f32,
            alpha: f32
        }

        struct Camera {
            transform: mat4x4<f32>,
            projection: mat4x4<f32>,
            projection_inverse: mat4x4<f32>,
            near: f32,
            far: f32,
        }

        struct Environment {
            surface: vec2<u32>,
            volume: u32,
            tile: u32,
            camera: Camera,
            light: vec3<f32>,
            light_: f32,
            settings: Settings
        }
        "
        .to_string()
    }

    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 512,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self { binding, buffer }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn update(&self, gpu: &Gpu, controller: &Controller) {
        gpu.queue().write_buffer(
            &self.buffer,
            0,
            &[
                bytes_of(&[controller.width(), controller.height()]),
                bytes_of(&[controller.volume(), controller.tile()]),
                bytes_of(&controller.camera().transform()),
                bytes_of(&controller.camera().projection()),
                bytes_of(&controller.camera().projection().inverse()),
                bytes_of(&controller.camera().near()),
                bytes_of(&controller.camera().far()),
                bytes_of(&0u64),
                bytes_of(&controller.light().direction()),
                bytes_of(&0u32),
                bytes_of(&controller.settings().streamline_radius),
                bytes_of(&controller.settings().direct_light),
                bytes_of(&controller.settings().culling_threshold),
                bytes_of(&controller.settings().alpha),
            ]
            .concat(),
        );
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
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
