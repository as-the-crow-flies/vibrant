use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::LineSet,
        scalar::{R32Float, ScalarTexture3D},
    },
    controller::settings::VoxelizationSetting,
    gpu::Gpu,
    renderer::{environment::Environment, wgsl},
    surface::{occupancy::Occupancy, Frame},
};

pub struct PopulatePipeline {
    populate_tube: ComputePipeline,
    populate_line: ComputePipeline,
    populate_box: ComputePipeline,
}

impl PopulatePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = &gpu.pipeline_layout(&[
            &Occupancy::layout(gpu, false),
            &ScalarTexture3D::<R32Float>::layout(gpu),
            &LineSet::layout(gpu, true),
            &Environment::layout(gpu),
        ]);

        let populate_source = include_str!("populate.wgsl");

        Self {
            populate_tube: gpu.compute(
                "Populate::Populate::Tube",
                layout,
                &gpu.shader(&(wgsl::voxelize::TUBE.to_owned() + populate_source)),
            ),
            populate_line: gpu.compute(
                "Populate::Populate::Line",
                layout,
                &gpu.shader(&(wgsl::voxelize::LINE.to_owned() + populate_source)),
            ),
            populate_box: gpu.compute(
                "Populate::Populate::Box",
                layout,
                &gpu.shader(&(wgsl::voxelize::BOX.to_owned() + populate_source)),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        setting: VoxelizationSetting,
        tractogram: &LineSet,
    ) {
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Populate"),
            ..Default::default()
        });

        pass.set_pipeline(match setting {
            VoxelizationSetting::Line => &self.populate_line,
            VoxelizationSetting::Box => &self.populate_box,
            VoxelizationSetting::Tube => &self.populate_tube,
        });

        pass.set_bind_group(0, frame.occupancy().binding(false), &[]);
        pass.set_bind_group(1, frame.occupancy().texture().binding(), &[]);
        pass.set_bind_group(2, tractogram.binding(true), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
