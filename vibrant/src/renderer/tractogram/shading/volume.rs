use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::scalar::{R8Unorm, Rgba8Unorm, ScalarTexture3D},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::Color, Frame},
};

pub struct VolumeRenderPipeline {
    pipeline: ComputePipeline,
}

impl VolumeRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.compute(
                "Volume",
                &gpu.pipeline_layout(&[
                    &ScalarTexture3D::<R8Unorm>::layout(gpu),
                    &ScalarTexture3D::<Rgba8Unorm>::layout(gpu),
                    &Environment::layout(gpu),
                    &Color::layout_write(gpu),
                ]),
                &gpu.shader(include_str!("volume.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Render"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.density().density().binding(), &[]);
        pass.set_bind_group(1, frame.density().color().binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, frame.color().binding(), &[]);

        pass.dispatch_workgroups(
            frame.color().width().div_ceil(8),
            frame.color().height().div_ceil(8),
            1,
        );
    }
}
