use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{asset::line::LineBuffer, gpu::Gpu, renderer::environment::Environment};

pub struct LineSelectionPipeline {
    selection: ComputePipeline,
}

impl LineSelectionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        // TODO: this should be generated from the controller settings, not hardcoded
        let source = include_str!("shapes/square.wgsl");
        Self {
            selection: gpu.compute(
                "Selection",
                &gpu.pipeline_layout(&[&LineBuffer::layout(gpu, false), &Environment::layout(gpu)]),
                &gpu.shader(source),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, line: &LineBuffer, environment: &Environment) {
        self.selection(cmd, line, environment);
    }

    fn selection(&self, cmd: &mut CommandEncoder, line: &LineBuffer, environment: &Environment) {
        line.clear_length(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Selection"),
            ..Default::default()
        });

        pass.set_pipeline(&self.selection);
        pass.set_bind_group(0, line.binding(false), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.dispatch_workgroups(line.n_lines().div_ceil(32), 1, 1);
    }
}
