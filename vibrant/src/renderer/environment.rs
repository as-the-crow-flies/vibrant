use std::any::type_name;

use bytemuck::bytes_of;
use glam::{Mat4, Vec2, Vec4};
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

    // Peripheral pass (foveated rendering, low-res)
    peripheral_binding: BindGroup,
    peripheral_buffer: Buffer,

    // Focus pass (foveated rendering, high-res sub-frustum)
    focus_binding: BindGroup,
    focus_buffer: Buffer,

    // TAA
    previous_projection: Mat4,
    frame_index: u32,
}

fn create_buffer_and_binding(gpu: &Gpu, layout: &BindGroupLayout) -> (Buffer, BindGroup) {
    let label = Some(type_name::<Environment>());

    let buffer = gpu.device().create_buffer(&BufferDescriptor {
        label,
        size: 512,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
        label,
        layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            }),
        }],
    });

    (buffer, binding)
}

/// Build a sub-frustum projection that zooms into a region around (mx, my) in NDC.
fn sub_frustum_projection(projection: Mat4, mx: f32, my: f32, r: f32) -> Mat4 {
    let scale = 1.0 / r;
    let sub_frustum = Mat4::from_cols(
        Vec4::new(scale, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(-mx * scale, -my * scale, 0.0, 1.0),
    );
    sub_frustum * projection
}

impl Environment {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = Self::layout(gpu);
        let (buffer, binding) = create_buffer_and_binding(gpu, &layout);
        let (peripheral_buffer, peripheral_binding) = create_buffer_and_binding(gpu, &layout);
        let (focus_buffer, focus_binding) = create_buffer_and_binding(gpu, &layout);

        Self {
            binding,
            buffer,
            peripheral_binding,
            peripheral_buffer,
            focus_binding,
            focus_buffer,
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

    pub fn peripheral_binding(&self) -> &BindGroup {
        &self.peripheral_binding
    }

    pub fn focus_binding(&self) -> &BindGroup {
        &self.focus_binding
    }

    pub fn update(&mut self, gpu: &Gpu, controller: &Controller) {
        let settings = controller.settings();
        let camera = controller.camera();
        let is_taa = settings.effective_aa_mode == AntiAliasingMode::TAA;

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

        // Write main environment (normal resolution — used by compute passes, post, AA)
        let data = Self::build_data(
            settings.render_width,
            settings.render_height,
            projection,
            controller,
            &self.previous_projection,
            &jitter_uv,
        );
        gpu.queue().write_buffer(&self.buffer, 0, &data);

        // Write peripheral + focus environments (foveated rendering)
        if settings.foveated {
            let peripheral_data = Self::build_data(
                settings.peripheral_width(),
                settings.peripheral_height(),
                projection,
                controller,
                &self.previous_projection,
                &jitter_uv,
            );
            gpu.queue().write_buffer(&self.peripheral_buffer, 0, &peripheral_data);
            let focus_projection = sub_frustum_projection(
                projection,
                settings.foveated_mouse_x,
                settings.foveated_mouse_y,
                settings.foveated_focus_radius,
            );

            let focus_width = settings.focus_width();
            let focus_height = settings.focus_height();

            let focus_jitter_uv = jitter / Vec2::new(
                focus_width as f32,
                focus_height as f32,
            );

            let focus_data = Self::build_data(
                focus_width,
                focus_height,
                focus_projection,
                controller,
                &self.previous_projection,
                &focus_jitter_uv,
            );
            gpu.queue().write_buffer(&self.focus_buffer, 0, &focus_data);
        }

        // Save current projection for next frame's reprojection.
        self.previous_projection = projection;
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    fn build_data(
        surface_width: u32,
        surface_height: u32,
        projection: Mat4,
        controller: &Controller,
        previous_projection: &Mat4,
        jitter_uv: &Vec2,
    ) -> Vec<u8> {
        let settings = controller.settings();
        [
            bytes_of(&[surface_width, surface_height, settings.volume]),
            bytes_of(&controller.time()),
            bytes_of(&controller.camera().transform()),
            bytes_of(&projection),
            bytes_of(&projection.inverse()),
            bytes_of(&controller.camera().near()),
            bytes_of(&controller.camera().far()),
            bytes_of(&0u64),
            bytes_of(&controller.segment().position()),
            bytes_of(&controller.segment().radius()),
            bytes_of(&controller.light().direction()),
            bytes_of(&0u32),
            bytes_of(&settings.radius),
            bytes_of(&settings.lighting),
            bytes_of(&settings.direct_light),
            bytes_of(&settings.tangent_color),
            bytes_of(&settings.shadows),
            bytes_of(&settings.alpha),
            bytes_of(&settings.level),
            bytes_of(&settings.smoothing),
            bytes_of(&(settings.culling as u32)),
            bytes_of(&settings.crop_start),
            bytes_of(&settings.crop_end),
            bytes_of(&settings.crop_x_start),
            bytes_of(&settings.crop_x_end),
            bytes_of(&settings.crop_y_start),
            bytes_of(&settings.crop_y_end),
            bytes_of(&settings.crop_z_start),
            bytes_of(&settings.crop_z_end),
            bytes_of(&settings.plane),
            bytes_of(&settings.bloom_threshold),
            bytes_of(&settings.bloom_soft_knee),
            bytes_of(&settings.bloom_intensity),
            bytes_of(&settings.smaa_threshold),
            bytes_of(&settings.smaa_max_search_steps),
            // TAA settings (appended after SMAA fields)
            bytes_of(&settings.taa_blend_factor),
            bytes_of(&settings.taa_clamp_sigma),
            // Padding: Settings ends at offset 356; mat4x4 needs 16-byte alignment → pad to 368
            bytes_of(&[0u32; 3]),
            // Previous frame's projection matrix for TAA reprojection (64 bytes)
            bytes_of(previous_projection),
            // Current frame's jitter offset in UV space (8 bytes)
            bytes_of(jitter_uv),
            // Foveated rendering parameters
            bytes_of(&[settings.foveated_mouse_x, settings.foveated_mouse_y]),
            bytes_of(&settings.foveated_focus_radius),
            bytes_of(&settings.foveated_blend_width),
        ]
        .concat()
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
