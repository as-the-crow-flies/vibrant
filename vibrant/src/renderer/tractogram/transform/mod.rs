use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{asset::line::LineSet, gpu::Gpu};

pub struct TransformPipeline {
    transform: ComputePipeline,
    adjacency: ComputePipeline,
}

impl TransformPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: gpu.compute(
                "Transform",
                &gpu.pipeline_layout(&[&LineSet::layout(gpu, false)]),
                &gpu.shader(include_str!("transform.wgsl")),
            ),
            adjacency: gpu.compute(
                "Adjacency",
                &gpu.pipeline_layout(&[&LineSet::layout(gpu, false)]),
                &gpu.shader(include_str!("adjacency.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, tractogram: &LineSet) {
        self.transform(cmd, tractogram);
        self.adjacency(cmd, tractogram);
    }

    fn transform(&self, cmd: &mut CommandEncoder, tractogram: &LineSet) {
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Transform"),
            ..Default::default()
        });

        pass.set_pipeline(&self.transform);
        pass.set_bind_group(0, tractogram.binding(false), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }

    fn adjacency(&self, cmd: &mut CommandEncoder, tractogram: &LineSet) {
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Adjacency"),
            ..Default::default()
        });

        pass.set_pipeline(&self.adjacency);
        pass.set_bind_group(0, tractogram.binding(false), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
