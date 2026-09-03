use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, ShaderStages,
};

use crate::{controller::Controller, gpu::Gpu};

/// Tractography-only render parameters, pulled out of the `Environment` uniform.
/// Uploaded straight from `controller.settings()` each frame; the line passes
/// (`crop`, `occupancy`, `cull`, `populate`, `render`) bind it instead of the
/// camera-oriented `Environment`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TractographySettings {
    /// Voxel-grid resolution the acceleration structure is built at.
    pub volume: u32,
    pub culling: u32,
    pub render_mode: u32,
    pub line_crop: u32,
    pub radius: f32,
    pub alpha: f32,
    pub ambient_light: f32,
    pub direct_light: f32,
    pub smoothing: f32,
    pub crop_start: f32,
    pub crop_end: f32,
    pub line_roughness: f32,
    pub line_specular: f32,
}

pub struct Tractography {
    buffer: Buffer,
    binding: BindGroup,
}

impl Tractography {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&TractographySettings::zeroed()),
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

        Self { buffer, binding }
    }

    pub fn update(&self, gpu: &Gpu, controller: &Controller) {
        let s = controller.settings();

        let settings = TractographySettings {
            volume: s.volume,
            culling: s.culling as u32,
            render_mode: s.render_mode as u32,
            line_crop: s.line_crop as u32,
            radius: s.radius,
            alpha: s.alpha,
            ambient_light: s.ambient_light,
            direct_light: s.direct_light,
            smoothing: s.smoothing,
            crop_start: s.crop_start,
            crop_end: s.crop_end,
            line_roughness: s.line_roughness,
            line_specular: s.line_specular,
        };

        gpu.queue()
            .write_buffer(&self.buffer, 0, bytes_of(&settings));
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
