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
            ambient_occlusion_samples: u32,
            streamline_radius: f32,
            direct_light: f32,
            gradient_factor: f32,
            opacity_factor: f32,
            step_size: f32,
            grad_size: f32,
            min_value: f32,
            shading_level: f32,
            cull_level: f32,
            balancing: u32
        }

        struct Camera {
            transform: mat4x4<f32>,
            projection: mat4x4<f32>,
            projection_inverse: mat4x4<f32>,
            near: f32,
            far: f32,
            padding: vec2<f32>
        }

        struct Environment {
            camera: Camera,
            light: vec3<f32>,
            padding: u32,
            settings: Settings
        }
        "
        .to_string()
    }

    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 272,
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
                bytes_of(&controller.camera().transform()),
                bytes_of(&controller.camera().projection()),
                bytes_of(&controller.camera().projection().inverse()),
                bytes_of(&controller.camera().near()),
                bytes_of(&controller.camera().far()),
                bytes_of(&0u64),
                bytes_of(&controller.light().direction()),
                bytes_of(&0u32),
                bytes_of(&controller.settings().ambient_occlusion_samples),
                bytes_of(&controller.settings().streamline_radius),
                bytes_of(&controller.settings().direct_light),
                bytes_of(&controller.settings().gradient_factor),
                bytes_of(&controller.settings().opacity_factor),
                bytes_of(&controller.settings().step_size),
                bytes_of(&controller.settings().grad_size),
                bytes_of(&controller.settings().min_value),
                bytes_of(&controller.settings().shading_level),
                bytes_of(&controller.settings().cull_level),
                bytes_of(&(controller.settings().balancing as u32)),
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
