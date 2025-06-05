use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{asset::tractogram::Tractogram, gpu::Gpu};

pub struct AdjacenyPipeline {
    pipeline: ComputePipeline,
}

impl AdjacenyPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.compute(
                "Adjacency",
                &gpu.pipeline_layout(&[&Tractogram::layout(gpu, false)]),
                &gpu.shader(include_str!("adjacency.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, tractogram: &Tractogram) {
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Adjaceny"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, tractogram.binding(false), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
