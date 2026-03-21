use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::line::LineBuffer,
    controller::{selection_volume::SelectionVolume, settings::Settings},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct LineSelectionPipeline {
    box_selection: ComputePipeline,
    sphere_selection: ComputePipeline,
}

impl LineSelectionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout =
            gpu.pipeline_layout(&[&LineBuffer::layout(gpu, false), &Environment::layout(gpu)]);

        // TODO: currently hardcoded to be two boxes, one small one customizable in the editor.
        //       need to make this dynamic through a menu...
        let square_source: String = include_str!("shapes/preamble.wgsl").to_string().replace(
            "//DISPATCH-INSERT-MARKER//",
            include_str!("shapes/square.wgsl"),
        );

        let sphere_source: String = include_str!("shapes/preamble.wgsl").to_string().replace(
            "//DISPATCH-INSERT-MARKER//",
            include_str!("shapes/sphere.wgsl"),
        );

        Self {
            box_selection: gpu.compute("Box", &layout, &gpu.shader(&square_source)),
            sphere_selection: gpu.compute("Sphere", &layout, &gpu.shader(&sphere_source)),
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
            SelectionVolume::Sphere => &self.sphere_selection,
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
