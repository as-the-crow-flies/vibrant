use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::scalar::{R8Unorm, ScalarTexture3D},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::SurfaceBuffer,
};

pub struct TractogramOcclusionPipeline {
    occlusion: ComputePipeline,
}

impl TractogramOcclusionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            occlusion: gpu.compute(
                "Occlusion::Occlusion",
                &gpu.pipeline_layout(&[
                    &ScalarTexture3D::<R8Unorm>::layout(gpu),
                    &ScalarTexture3D::<R8Unorm>::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("occlusion.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        buffer: &SurfaceBuffer,
        environment: &Environment,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        let n = buffer.occlusion().volume().width().div_ceil(8);

        pass.set_pipeline(&self.occlusion);
        pass.set_bind_group(0, buffer.density().density().binding(), &[]);
        pass.set_bind_group(1, buffer.occlusion().volume().binding_write(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);
    }
}
