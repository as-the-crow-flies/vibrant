use egui::Rect;
use wgpu::{
    CommandEncoder, ComputePassDescriptor, ComputePipeline, RenderPassDescriptor, RenderPipeline,
};

use crate::{
    asset::{hdri::HdriBuffer, radiance::gaussian::GaussianRadianceBuffer, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

const VMM_SIZE: u32 = 16;

pub struct GaussianVolumeRenderer {
    cascade: Vec<ComputePipeline>,
    trace: RenderPipeline,
}

impl GaussianVolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = gpu.pipeline_layout(&[
            &GaussianRadianceBuffer::layout_mipmap(gpu),
            &PhysicalVolume::layout_read(gpu),
            &Environment::layout(gpu),
            &HdriBuffer::layout(gpu),
        ]);

        let common = include_str!("common.wgsl").replace("#VMM_SIZE", &VMM_SIZE.to_string());

        let cascade_src = include_str!("cascade.wgsl").to_string() + &common;
        let trace_src = include_str!("trace.wgsl").to_string() + &common;

        Self {
            cascade: (0..GaussianRadianceBuffer::LEVELS)
                .map(|cascade| {
                    gpu.compute(
                        "GaussianVolumeCascade",
                        &layout,
                        &gpu.shader(&cascade_template(&cascade_src, cascade)),
                    )
                })
                .collect(),
            trace: gpu.quad(
                "GaussianVolumeTrace",
                &gpu.pipeline_layout(&[
                    &GaussianRadianceBuffer::layout(gpu),
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(&trace_src),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
        viewport: Rect,
        recompute: bool,
    ) {
        if recompute {
            self.radiance(cmd, environment, hdri, radiance, volume);
        }
        self.trace(cmd, environment, hdri, frame, radiance, volume, viewport);
    }

    fn radiance(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        for cascade in (0..GaussianRadianceBuffer::LEVELS as usize).rev() {
            let probes = radiance.probes(cascade);
            let batch = probes_per_workgroup(cascade as u32);

            pass.set_pipeline(&self.cascade[cascade]);
            pass.set_bind_group(0, radiance.binding_mipmap(cascade), &[]);
            pass.dispatch_workgroups(probes.x.div_ceil(batch), probes.y, probes.z);
        }
    }

    fn trace(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
        viewport: Rect,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment())],
            ..Default::default()
        });

        pass.set_viewport(
            viewport.min.x,
            viewport.min.y,
            viewport.width(),
            viewport.height(),
            0.0,
            1.0,
        );

        pass.set_pipeline(&self.trace);

        pass.set_bind_group(0, radiance.binding(), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}

fn threads_per_probe(cascade: u32) -> u32 {
    match cascade {
        5 => 1024,
        4 => 256,
        3 => 64,
        2 => 16,
        1 => 4,
        _ => 1,
    }
}

// wgpu reports `max_compute_workgroup_storage_size` = 32768 bytes on this
// adapter (Gpu::new requests the adapter's maximum) — every cascade's
// workgroup arrays in `cascade.wgsl` must fit inside that budget.
const MAX_WORKGROUP_STORAGE_BYTES: u32 = 32768;

// Probes packed into one workgroup along x at dispatch time — must match
// WORKGROUP_SIZE / THREADS in `cascade_template` for each cascade.
//
// Cascades 0-2 batch several probes into a single workgroup (SUBGROUPS == 1
// for all three), and every probe gets its own copy of the four
// PROBES * VMM_SIZE workgroup arrays in `cascade.wgsl`
// (VMM/PHI/VMM_PRIOR/PHI_PRIOR, 16 bytes per vec4<f32>) plus two smaller
// SUBGROUPS * VMM_SIZE partial-reduction arrays — 16 * VMM_SIZE * (4 * PROBES
// + 2) bytes total. Left at a fixed batch size that grows linearly with
// VMM_SIZE: cascade 0's batch of 64 probes already overflows the device's
// workgroup storage limit at VMM_SIZE = 8 (33024 > 32768 bytes), so its
// pipeline silently fails to compile and the finest cascade (lod 0) is never
// written. Cap each cascade's batch to the largest power of two that keeps
// its footprint under that limit.
fn probes_per_workgroup(cascade: u32) -> u32 {
    let base = match cascade {
        2 => 4,
        1 => 16,
        0 => 64,
        _ => 1,
    };

    if base == 1 {
        return 1;
    }

    let budget = (MAX_WORKGROUP_STORAGE_BYTES / (16 * VMM_SIZE)).saturating_sub(2) / 4;

    base.min(prev_pow2(budget.max(1)))
}

fn prev_pow2(n: u32) -> u32 {
    1 << (31 - n.leading_zeros())
}

fn cascade_template(src: &str, cascade: u32) -> String {
    src.replace("#CASCADE", &cascade.to_string())
        .to_string()
        .replace(
            "#WORKGROUP_SIZE",
            &match cascade {
                5 => 1024,
                4 => 256,
                _ => threads_per_probe(cascade) * probes_per_workgroup(cascade),
            }
            .to_string(),
        )
        .replace(
            "#SUBGROUPS",
            &match cascade {
                5 => 32,
                4 => 8,
                3 => 2,
                _ => 1,
            }
            .to_string(),
        )
        .replace("#THREADS", &threads_per_probe(cascade).to_string())
        .replace(
            "#SAMPLES",
            &match cascade {
                5 => 4096,
                4 => 1024,
                3 => 256,
                2 => 64,
                1 => 16,
                _ => 4,
            }
            .to_string(),
        )
        .to_string()
}
