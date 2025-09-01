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

    pub fn from_controller(gpu: &Gpu, controller: &Controller) -> Self {
        let environment = Environment::new(&gpu);
        environment.update(&gpu, controller);
        return environment;
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn update(&self, gpu: &Gpu, controller: &Controller) {
        gpu.queue().write_buffer(
            &self.buffer,
            0,
            &[
                bytes_of(&[
                    controller.settings().width,
                    controller.settings().height,
                    controller.settings().volume,
                    0,
                ]),
                bytes_of(&controller.camera().transform()),
                bytes_of(&controller.camera().projection()),
                bytes_of(&controller.camera().projection().inverse()),
                bytes_of(&controller.camera().near()),
                bytes_of(&controller.camera().far()),
                bytes_of(&0u64),
                bytes_of(&controller.segment().position()),
                bytes_of(&controller.segment().radius()),
                bytes_of(&controller.light().direction()),
                bytes_of(&0u32),
                bytes_of(&controller.settings().radius),
                bytes_of(&controller.settings().lighting),
                bytes_of(&controller.settings().direct_light),
                bytes_of(&controller.settings().tangent_color),
                bytes_of(&controller.settings().shadows),
                bytes_of(&controller.settings().alpha),
                bytes_of(&controller.settings().level),
                bytes_of(&controller.settings().smoothing),
                bytes_of(&(controller.settings().culling as u32)),
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
                    visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT,
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
