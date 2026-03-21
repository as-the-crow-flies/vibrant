use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::line::LineBuffer,
    controller::{selection_volume::SelectionVolume, settings::Settings},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct LineSelectionPipeline {
    selection_pipeline: ComputePipeline,
}

impl LineSelectionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout =
            gpu.pipeline_layout(&[&LineBuffer::layout(gpu, false), &Environment::layout(gpu)]);

        // TODO: currently hardcoded to be two boxes, one small one customizable in the editor.
        //       need to make this dynamic through a menu...
        let preamble_source = include_str!("shapes/preamble.wgsl")
            .to_string()
            .replace(
                "//DISPATCH-INSERT-MARKER//",
                include_str!("shapes/square.wgsl"),
            )
            .replace(
                "//DISPATCH-INSERT-MARKER//",
                include_str!("shapes/sphere.wgsl"),
            );
        // .replace(
        //     "//DISPATCH-PARTIAL-CALL-MARKER//",
        //     "
        //     in_square_volume_segment(ENVIRONMENT.settings.selection_scale, ENVIRONMENT.settings.selection_offset_x, ENVIRONMENT.settings.selection_offset_y, ENVIRONMENT.settings.selection_offset_z, idx) ||
        //     in_square_volume_segment(0.5, 0.125, 0.125, 0.125, idx)
        //     ",
        // );

        Self {
            selection_pipeline: gpu.compute(
                "Selection Pipeline",
                &layout,
                &gpu.shader(&preamble_source),
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
        let pipeline: &ComputePipeline = match settings.selection_volume {
            SelectionVolume::None => return,
            _ => &self.selection_pipeline,
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
