use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::texture::{MipTexture3D, R32Float},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::Frame,
};

pub struct AmbientOcclusionPipeline {
    occlusion: ComputePipeline,
}

impl AmbientOcclusionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            occlusion: gpu.compute(
                "Occlusion::Ambient",
                &gpu.pipeline_layout(&[
                    &MipTexture3D::<R32Float>::layout(gpu),
                    &MipTexture3D::<R32Float>::layout(gpu),
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("ambient.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occlusion"),
            ..Default::default()
        });

        let n = frame.occlusion().ambient().resolution().div_ceil(4);

        pass.set_pipeline(&self.occlusion);
        pass.set_bind_group(0, frame.occupancy().pyramid().binding(), &[]);
        pass.set_bind_group(1, frame.culling().pyramid().binding(), &[]);
        pass.set_bind_group(2, frame.occlusion().ambient().binding_write(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);
    }
}
