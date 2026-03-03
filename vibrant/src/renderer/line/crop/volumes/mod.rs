use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::line::LineBuffer,
    controller::{selection_volume::SelectionVolume, settings::Settings},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct LineSelectionPipeline {
    box_selection: ComputePipeline,
}

impl LineSelectionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout =
            gpu.pipeline_layout(&[&LineBuffer::layout(gpu, false), &Environment::layout(gpu)]);

        Self {
            box_selection: gpu.compute(
                "Selection (Box)",
                &layout,
                &gpu.shader(include_str!("shapes/square.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        line: &LineBuffer,
        environment: &Environment,
        settings: &Settings,
    ) {
        let pipeline = match settings.selection_volume {
            SelectionVolume::None => return,
            SelectionVolume::Box => &self.box_selection,
        };

        self.selection(cmd, line, environment, pipeline);
    }

    fn selection(
        &self,
        cmd: &mut CommandEncoder,
        line: &LineBuffer,
        environment: &Environment,
        pipeline: &ComputePipeline,
    ) {
        line.clear_length(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Selection"),
            ..Default::default()
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, line.binding(false), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.dispatch_workgroups(line.n_lines().div_ceil(32), 1, 1);
    }
}
