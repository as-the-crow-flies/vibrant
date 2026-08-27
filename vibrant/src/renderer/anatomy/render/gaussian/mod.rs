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

pub struct GaussianVolumeRenderer {
    hdri: ComputePipeline,
    cascade: Vec<ComputePipeline>,
    trace: RenderPipeline,
}

impl GaussianVolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("common.wgsl");

        let hdri_src = include_str!("hdri.wgsl").to_string() + &common;
        let cascade_src = include_str!("cascade.wgsl").to_string() + &common;
        let trace_src = include_str!("trace.wgsl").to_string() + &common;

        Self {
            hdri: gpu.compute(
                "GaussianVolumeHdri",
                &gpu.pipeline_layout(&[
                    &GaussianRadianceBuffer::layout_hdri(gpu),
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                &gpu.shader(&hdri_src),
            ),
            cascade: (0..GaussianRadianceBuffer::LEVELS)
                .map(|cascade| {
                    gpu.compute(
                        "GaussianVolumeCascade",
                        &gpu.pipeline_layout(&[
                            &GaussianRadianceBuffer::layout_mipmap(gpu),
                            &PhysicalVolume::layout_read(gpu),
                            &Environment::layout(gpu),
                            &HdriBuffer::layout(gpu),
                        ]),
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
            self.hdri(cmd, environment, hdri, radiance, volume);
            self.radiance(cmd, environment, hdri, radiance, volume);
        }

        self.trace(cmd, environment, hdri, frame, radiance, volume, viewport);
    }

    pub fn hdri(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("GaussianVolumeHdri"),
            ..Default::default()
        });

        pass.set_pipeline(&self.hdri);
        pass.set_bind_group(0, radiance.binding_hdri(), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        pass.dispatch_workgroups(1, 1, 1);
    }

    pub fn radiance(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
    ) {
        for cascade in (0..GaussianRadianceBuffer::LEVELS as usize).rev() {
            self.cascade(cmd, environment, hdri, radiance, volume, cascade);
        }
    }

    pub fn cascade(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
        cascade: usize,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(&format!("Cascade {}", cascade)),
            ..Default::default()
        });

        pass.set_pipeline(&self.cascade[cascade]);
        pass.set_bind_group(0, radiance.binding_mipmap(cascade), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        let probes = radiance.probes(cascade);
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

fn cascade_template(src: &str, cascade: u32) -> String {
    src.replace("#CASCADE", &cascade.to_string())
        .to_string()
        .replace(
            "#WORKGROUP",
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
