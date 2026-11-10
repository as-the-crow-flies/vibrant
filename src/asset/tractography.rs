use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, ShaderStages,
};

use crate::{asset::hdri::LOOKUP_JITTER, controller::Controller, gpu::Gpu};

/// Tractography-only render parameters, pulled out of the `Environment` uniform.
/// Uploaded straight from `controller.settings()` each frame; the line passes
/// (`crop`, `occupancy`, `populate`, `render`) bind it instead of the
/// camera-oriented `Environment`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TractographySettings {
    /// Voxel-grid resolution the acceleration structure is built at.
    pub volume: u32,
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
    pub line_translucency: f32,
    /// Line extinction per unit volume fraction, per mm: 1 / (2 r).
    pub line_extinction: f32,
    /// Probe lookup jitter in probe cells; 0 unless accumulating.
    pub lookup_jitter: f32,
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

    pub fn update(&self, gpu: &Gpu, controller: &Controller, mm_per_voxel: f32) {
        let s = controller.settings();

        let settings = TractographySettings {
            volume: s.volume,
            line_crop: s.line_crop as u32,
            radius: s.radius / mm_per_voxel,
            alpha: s.alpha,
            ambient_light: s.ambient_light,
            direct_light: s.direct_light,
            smoothing: s.smoothing,
            crop_start: s.crop_start,
            crop_end: s.crop_end,
            line_roughness: s.line_roughness,
            line_specular: s.line_specular,
            line_translucency: s.line_translucency,
            // Randomly oriented cylinders: mean projected area is a quarter of
            // the surface (Cauchy), so σ = f / (2r).
            line_extinction: 1.0 / (2.0 * s.radius.max(1e-3)),
            lookup_jitter: if s.accumulate && s.lookup_jitter {
                LOOKUP_JITTER
            } else {
                0.0
            },
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
