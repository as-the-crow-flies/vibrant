use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{asset::line::LineSet, gpu::Gpu, renderer::environment::Environment};

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
                    &LineSet::layout_raw(gpu),
                    &LineSet::layout(gpu, false),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("transform.wgsl")),
            ),
            adjacency: gpu.compute(
                "Adjacency",
                &gpu.pipeline_layout(&[&LineSet::layout(gpu, false)]),
                &gpu.shader(include_str!("adjacency.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, line: &LineSet) {
        self.transform(cmd, line, environment);
        self.adjacency(cmd, line);
    }

    fn transform(&self, cmd: &mut CommandEncoder, line: &LineSet, environment: &Environment) {
        line.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Transform"),
            ..Default::default()
        });

        pass.set_pipeline(&self.transform);
        pass.set_bind_group(0, line.binding_raw(), &[]);
        pass.set_bind_group(1, line.binding(false), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }

    fn adjacency(&self, cmd: &mut CommandEncoder, line: &LineSet) {
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
