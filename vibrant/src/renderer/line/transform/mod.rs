use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    gpu::Gpu,
};

pub struct LineTransformPipeline {
    transform: ComputePipeline,
    adjacency: ComputePipeline,
}

impl LineTransformPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: gpu.compute(
                "Transform",
                &gpu.pipeline_layout(&[
                    &LineBuffer::layout(gpu, false),
                    &TransformBuffer::layout(gpu),
                ]),
                &gpu.shader(include_str!("transform.wgsl")),
            ),
            adjacency: gpu.compute(
                "Adjacency",
                &gpu.pipeline_layout(&[&LineBuffer::layout(gpu, false)]),
                &gpu.shader(include_str!("adjacency.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        line: &LineBuffer,
        transform: &TransformBuffer,
    ) {
        self.transform(cmd, line, transform);
        self.adjacency(cmd, line);
    }

    fn transform(&self, cmd: &mut CommandEncoder, line: &LineBuffer, transform: &TransformBuffer) {
        line.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Transform"),
            ..Default::default()
        });

        pass.set_pipeline(&self.transform);
        pass.set_bind_group(0, line.binding(false), &[]);
        pass.set_bind_group(1, transform.binding(), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }

    fn adjacency(&self, cmd: &mut CommandEncoder, line: &LineBuffer) {
        line.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Adjacency"),
            ..Default::default()
        });

        pass.set_pipeline(&self.adjacency);
        pass.set_bind_group(0, line.binding(false), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
