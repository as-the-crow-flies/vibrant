use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct LineTransformPipeline {
    transform: ComputePipeline,
}

impl LineTransformPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: gpu.compute(
                "Transform",
                &gpu.pipeline_layout(&[
                    &LineBuffer::layout(gpu, false),
                    &TransformBuffer::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("transform.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        line: &LineBuffer,
        transform: &TransformBuffer,
        environment: &Environment,
    ) {
        line.clear_offset(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Transform"),
            ..Default::default()
        });

        pass.set_pipeline(&self.transform);
        pass.set_bind_group(0, line.binding(false), &[]);
        pass.set_bind_group(1, transform.binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
