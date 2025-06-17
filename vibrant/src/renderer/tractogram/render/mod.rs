use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        scalar::{R8Uint, R8Unorm, ScalarTexture3D},
        tractogram::Tractogram,
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::Color, occupancy::Occupancy, Frame},
};

pub struct TractogramRenderPipeline {
    pipeline: ComputePipeline,
}

impl TractogramRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.compute(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &Tractogram::layout(gpu, true),
                    &Occupancy::layout(gpu, true),
                    &ScalarTexture3D::<R8Unorm>::layout(gpu),
                    &ScalarTexture3D::<R8Uint>::layout(gpu),
                    &ScalarTexture3D::<R8Unorm>::layout(gpu),
                    &Environment::layout(gpu),
                    &Color::layout_write(gpu),
                ]),
                &gpu.shader(include_str!("render.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Render"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, tractogram.binding(true), &[]);
        pass.set_bind_group(1, frame.occupancy().binding(true), &[]);
        pass.set_bind_group(2, frame.occupancy().texture().binding(), &[]);
        pass.set_bind_group(3, frame.density().count().binding(), &[]);
        pass.set_bind_group(4, frame.density().texture().binding(), &[]);
        pass.set_bind_group(5, environment.binding(), &[]);
        pass.set_bind_group(6, frame.color().binding(), &[]);

        pass.dispatch_workgroups(
            frame.color().width().div_ceil(8),
            frame.color().height().div_ceil(8),
            1,
        );
    }
}

mod test {
    #[test]
    fn test() {}
}
