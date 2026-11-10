use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ShaderStages, TextureSampleType, TextureViewDimension,
};

use glam::{Mat4, Vec2, Vec3};

use crate::{asset::noise::BlueNoise, controller::Controller, gpu::Gpu};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    // View -> world. Kept separate from the clip -> view inverse so
    // unprojection stays precise at narrow fields of view.
    transform: Mat4,
    projection_inverse: Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EnvironmentUniform {
    camera: CameraUniform,
    time: f32,
    sample: u32,
    noise: u32,
    _pad: u32,
}

/// Camera, clock and noise shared by every view-dependent pass.
pub struct Environment {
    binding: BindGroup,
    buffer: Buffer,
    _noise: BlueNoise,
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

        let noise = BlueNoise::new(gpu);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(noise.view()),
                },
            ],
        });

        Self {
            binding,
            buffer,
            _noise: noise,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    /// `jitter` is a sub-pixel camera offset in pixel units (e.g. a Halton
    /// sequence in `[-0.5, 0.5]`), baked into the uploaded projection matrix so
    /// every view-dependent pass (volume trace, lines) gets temporal
    /// anti-aliasing for free while frames accumulate. Pass `Vec2::ZERO` when
    /// not accumulating. `sample` is the noise index of this frame.
    pub fn update(&self, gpu: &Gpu, controller: &Controller, jitter: Vec2, sample: u32) {
        let offset = Vec3::new(
            jitter.x * 2.0 / controller.settings().width as f32,
            jitter.y * 2.0 / controller.settings().height as f32,
            0.0,
        );
        let projection = Mat4::from_translation(offset) * controller.camera().projection();

        let uniform = EnvironmentUniform {
            camera: CameraUniform {
                transform: controller.camera().transform(),
                projection_inverse: projection.inverse(),
            },
            time: controller.time(),
            sample,
            noise: controller.settings().noise as u32,
            _pad: 0,
        };

        gpu.queue()
            .write_buffer(&self.buffer, 0, bytes_of(&uniform));
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
    }
}
