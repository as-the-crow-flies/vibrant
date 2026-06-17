use std::iter::zip;

use egui::Rect;
use wgpu::{
    CommandEncoder, ComputePassDescriptor, ComputePipeline, RenderPassDescriptor, RenderPipeline,
};

use crate::{
    asset::{hdri::HdriBuffer, radiance::RadianceCascadesBuffer, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct OctahedralVolumeRenderer {
    cascade: ComputePipeline,
    copy: ComputePipeline,
    trace: RenderPipeline,
}

impl OctahedralVolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("common.wgsl");

        Self {
            cascade: gpu.compute(
                "OctahedralVolumeCascade",
                &gpu.pipeline_layout(&[
                    &RadianceCascadesBuffer::layout_cascade(gpu),
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                &gpu.shader(&(common.to_string() + include_str!("cascade.wgsl"))),
            ),
            copy: gpu.compute(
                "OctahedralVolumeCopy",
                &gpu.pipeline_layout(&[&RadianceCascadesBuffer::layout_copy(gpu)]),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
            trace: gpu.quad(
                "OctahedralVolumeTrace",
                &gpu.pipeline_layout(&[
                    &RadianceCascadesBuffer::layout_read(gpu),
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(&(common.to_string() + include_str!("trace.wgsl"))),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        radiance: &RadianceCascadesBuffer,
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
        radiance: &RadianceCascadesBuffer,
        volume: &PhysicalVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.cascade);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        for (cascade, dim) in zip(radiance.binding_cascade(), radiance.size_cascade()).rev() {
            pass.set_bind_group(0, cascade, &[]);
            pass.dispatch_workgroups(
                dim.width.div_ceil(4),
                dim.height.div_ceil(4),
                dim.depth_or_array_layers.div_ceil(4),
            );
        }

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, radiance.binding_copy(), &[]);
        pass.dispatch_workgroups(
            radiance.size().width.div_ceil(4),
            radiance.size().height.div_ceil(4),
            radiance.size().depth_or_array_layers.div_ceil(4),
        );
    }

    fn trace(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        radiance: &RadianceCascadesBuffer,
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

        pass.set_bind_group(0, radiance.binding_read(), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
