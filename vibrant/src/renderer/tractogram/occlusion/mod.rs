use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::scalar::{R8Unorm, ScalarTexture3D},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::SurfaceBuffer,
};

pub struct OcclusionPipeline {
    occlusion: ComputePipeline,
}

impl OcclusionPipeline {
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
        frame: &SurfaceBuffer,
        environment: &Environment,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occlusion"),
            ..Default::default()
        });

        let n = frame.occlusion().volume().width().div_ceil(8);

        pass.set_pipeline(&self.occlusion);
        pass.set_bind_group(0, frame.density().volume().binding(), &[]);
        pass.set_bind_group(1, frame.occlusion().volume().binding_write(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);
    }
}
