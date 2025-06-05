use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        scalar::{R8Unorm, ScalarTexture3D},
        tractogram::Tractogram,
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{occupancy::Occupancy, SurfaceBuffer},
};

pub struct PopulatePipeline {
    populate: ComputePipeline,
}

impl PopulatePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            populate: gpu.compute(
                "Populate::Populate",
                &gpu.pipeline_layout(&[
                    &Occupancy::layout(gpu, false),
                    &ScalarTexture3D::<R8Unorm>::layout(gpu),
                    &Tractogram::layout(gpu, true),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(
                    &(include_str!("../../wgsl/voxelize.wgsl").to_owned()
                        + include_str!("populate.wgsl")),
                ),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &SurfaceBuffer,
        environment: &Environment,
        tractogram: &Tractogram,
    ) {
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Populate"),
            ..Default::default()
        });

        pass.set_pipeline(&self.populate);
        pass.set_bind_group(0, frame.occupancy().binding(false), &[]);
        pass.set_bind_group(1, frame.occupancy().occupancy().binding(), &[]);
        pass.set_bind_group(2, tractogram.binding(true), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
