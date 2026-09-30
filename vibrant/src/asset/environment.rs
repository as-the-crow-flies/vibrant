use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ShaderStages,
};

use glam::{Mat4, Vec2, Vec3};

use crate::{controller::Controller, gpu::Gpu};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    transform: Mat4,
    projection_inverse: Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EnvironmentUniform {
    camera: CameraUniform,
    time: f32,
    _pad: [f32; 3],
}

/// Camera + clock uniform shared by every view-dependent pass. Tractography
/// render parameters used to ride along here; they now live in
/// [`Tractography`](crate::asset::tractography::Tractography).
pub struct Environment {
    binding: BindGroup,
    buffer: Buffer,
}

impl Environment {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: std::mem::size_of::<EnvironmentUniform>() as u64,
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

    /// `jitter` is a sub-pixel camera offset in pixel units (e.g. a Halton
    /// sequence in `[-0.5, 0.5]`), baked into the uploaded projection matrix so
    /// every view-dependent pass (volume trace, lines) gets temporal
    /// anti-aliasing for free while frames accumulate. Pass `Vec2::ZERO` when
    /// not accumulating.
    pub fn update(&self, gpu: &Gpu, controller: &Controller, jitter: Vec2) {
        let settings = controller.settings();
        let width = settings.width.max(1) as f32;
        let height = settings.height.max(1) as f32;

        // NDC spans [-1, 1], so one pixel is `2 / dimension`.
        let offset = Vec3::new(jitter.x * 2.0 / width, jitter.y * 2.0 / height, 0.0);
        let projection = Mat4::from_translation(offset) * controller.camera().projection();

        let uniform = EnvironmentUniform {
            camera: CameraUniform {
                transform: controller.camera().transform(),
                projection_inverse: projection.inverse(),
            },
            time: controller.time(),
            _pad: [0.0; 3],
        };

        gpu.queue()
            .write_buffer(&self.buffer, 0, bytes_of(&uniform));
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
