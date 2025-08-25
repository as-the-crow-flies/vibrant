use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{asset::line::LineSet, gpu::Gpu, renderer::environment::Environment};

pub struct LineSegmentPipeline {
    segment: ComputePipeline,
}

impl LineSegmentPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            segment: gpu.compute(
                "Segment::Segment",
                &gpu.pipeline_layout(&[
                    &LineSet::layout_raw(gpu),
                    &LineSet::layout(gpu, false),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("segment.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, line: &LineSet) {
        line.clear_total_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        pass.set_pipeline(&self.segment);
        pass.set_bind_group(0, line.binding_raw(), &[]);
        pass.set_bind_group(1, line.binding(false), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(line.len().div_ceil(1024), 1, 1);
    }
}
