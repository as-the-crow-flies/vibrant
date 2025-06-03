use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        scalar::{R8Uint, R8Unorm, ScalarTexture3D},
        tractogram::Tractogram,
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::Color, occupancy::Occupancy, SurfaceBuffer},
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
                    &Tractogram::layout(gpu),
                    &Occupancy::layout(gpu, true),
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
        frame: &SurfaceBuffer,
        environment: &Environment,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, frame.occupancy().binding(true), &[]);
        pass.set_bind_group(2, frame.density().count().binding(), &[]);
        pass.set_bind_group(3, frame.density().volume().binding(), &[]);
        pass.set_bind_group(4, environment.binding(), &[]);
        pass.set_bind_group(5, frame.color().binding_write(), &[]);

        pass.dispatch_workgroups(
            frame.color().width().div_ceil(8),
            frame.color().height().div_ceil(8),
            1,
        );
    }
}
