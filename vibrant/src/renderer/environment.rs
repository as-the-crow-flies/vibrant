use std::any::type_name;

use bytemuck::bytes_of;
use glam::{Mat4, Vec2};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ShaderStages,
};

use crate::{
    controller::{camera::Camera, settings::AntiAliasingMode, Controller},
    gpu::Gpu,
};

pub struct Environment {
    binding: BindGroup,
    buffer: Buffer,
    
    // TAA
    previous_projection: Mat4,
    frame_index: u32,
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

        Self {
            binding,
            buffer,
            previous_projection: Mat4::IDENTITY,
            frame_index: 0,
        }
    }

    pub fn from_controller(gpu: &Gpu, controller: &Controller) -> Self {
        let mut environment = Environment::new(&gpu);
        environment.update(&gpu, controller);
        return environment;
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn update(&mut self, gpu: &Gpu, controller: &Controller) {
        let settings = controller.settings();
        let camera = controller.camera();
        let is_taa = settings.aa_mode == AntiAliasingMode::TAA;

        let jitter = if is_taa {
            Camera::jitter(self.frame_index)
        } else {
            Vec2::ZERO
        };

        // use jittered projection for TAA
        let projection = if is_taa {
            camera.projection_jittered(jitter)
        } else {
            camera.projection()
        };

        // jitter in UV space for the shader to undo
        let jitter_uv = jitter / Vec2::new(
            settings.render_width as f32,
            settings.render_height as f32,
        );

        gpu.queue().write_buffer(
            &self.buffer,
            0,
            &[
                bytes_of(&[
                    controller.settings().render_width,
                    controller.settings().render_height,
                    controller.settings().volume,
                ]),
                bytes_of(&controller.time()),
                bytes_of(&controller.camera().transform()),
                // bytes_of(&controller.camera().projection()),
                // bytes_of(&controller.camera().projection().inverse()),
                bytes_of(&projection),
                bytes_of(&projection.inverse()),
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
                bytes_of(&controller.settings().crop_start),
                bytes_of(&controller.settings().crop_end),
                bytes_of(&controller.settings().crop_x_start),
                bytes_of(&controller.settings().crop_x_end),
                bytes_of(&controller.settings().crop_y_start),
                bytes_of(&controller.settings().crop_y_end),
                bytes_of(&controller.settings().crop_z_start),
                bytes_of(&controller.settings().crop_z_end),
                bytes_of(&controller.settings().plane),
                bytes_of(&controller.settings().bloom_threshold),
                bytes_of(&controller.settings().bloom_soft_knee),
                bytes_of(&controller.settings().bloom_intensity),
                bytes_of(&controller.settings().smaa_threshold),
                bytes_of(&controller.settings().smaa_max_search_steps),
                // TAA settings (appended after SMAA fields)
                bytes_of(&controller.settings().taa_blend_factor),
                bytes_of(&controller.settings().taa_clamp_sigma),
                // Padding: Settings ends at offset 356; mat4x4 needs 16-byte alignment → pad to 368
                bytes_of(&[0u32; 3]),
                // Previous frame's projection matrix for TAA reprojection (64 bytes)
                bytes_of(&self.previous_projection),
                // Current frame's jitter offset in UV space (8 bytes)
                bytes_of(&jitter_uv),
            ]
            .concat(),
        );

        // Save current projection for next frame's reprojection.
        self.previous_projection = projection;
        self.frame_index = self.frame_index.wrapping_add(1);
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
