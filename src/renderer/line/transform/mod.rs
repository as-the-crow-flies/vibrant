use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{environment::Environment, line::LineBuffer, Asset},
    gpu::{Gpu, Lazy},
};

pub struct LineTransformPipeline {
    transform: Lazy<ComputePipeline>,
}

impl LineTransformPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: gpu.lazy("Transform", move |gpu| {
                gpu.compute(
                    "Transform",
                    &gpu.pipeline_layout(&[
                        &LineBuffer::layout_transform(gpu),
                        &Environment::layout(gpu),
                    ]),
                    &gpu.shader("renderer/line/transform/transform.wgsl"),
                )
            }),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, asset: &Asset, line: &LineBuffer) {
        line.clear_offset(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Transform"),
            ..Default::default()
        });

        pass.set_pipeline(&self.transform);
        pass.set_bind_group(0, line.binding_transform(), &[]);
        pass.set_bind_group(1, asset.environment.binding(), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
