use std::ops::{Div, Shr};

use egui::Rect;
use glam::UVec3;
use wgpu::{
    CommandEncoder, ComputePassDescriptor, ComputePipeline, RenderPassDescriptor, RenderPipeline,
};

use crate::{
    asset::{hdri::HdriBuffer, radiance::gaussian::GaussianRadianceBuffer, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

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

        let common = include_str!("common.wgsl").to_string();

        let cascade_src = include_str!("cascade.wgsl").to_string() + &common;
        let trace_src = include_str!("trace.wgsl").to_string() + &common;

        Self {
            cascade: (0..7)
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

        let cascade = 5;

        let probes = radiance
            .size()
            .shr(UVec3::splat(cascade as u32))
            .max(UVec3::ONE)
            .div(UVec3::new(4, 4, 2))
            .max(UVec3::ONE);

        pass.set_pipeline(&self.cascade[cascade]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        pass.set_bind_group(0, &radiance.bindings_mipmap()[cascade], &[]);

        pass.dispatch_workgroups(probes.x, probes.y, probes.z);
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

fn cascade_template(src: &str, cascade: usize) -> String {
    src.replace("#CASCADE", &cascade.to_string())
        .to_string()
        .replace(
            "#WORKGROUP_SIZE",
            &match cascade {
                5 => 1024,
                4 => 256,
                3 => 64,
                _ => 32,
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
        .replace(
            "#SAMPLES",
            &match cascade {
                5 => 4096,
                4 => 1024,
                3 => 256,
                _ => 128,
            }
            .to_string(),
        )
        .to_string()
}
