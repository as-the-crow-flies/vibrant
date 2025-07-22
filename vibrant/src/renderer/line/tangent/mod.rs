use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::texture::{MipTexture3D, R32Float, Rgba8Unorm},
    gpu::Gpu,
    surface::{tangent::TangentBuffer, Frame},
};

pub struct LineTangentPipeline {
    x: ComputePipeline,
    y: ComputePipeline,
    z: ComputePipeline,
}

impl LineTangentPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            x: gpu.compute(
                "Tangent::X",
                &gpu.pipeline_layout(&[
                    &TangentBuffer::layout(gpu),
                    &MipTexture3D::<R32Float>::layout(gpu),
                ]),
                &gpu.shader(include_str!("x.wgsl")),
            ),
            y: gpu.compute(
                "Tangent::Y",
                &gpu.pipeline_layout(&[&TangentBuffer::layout(gpu)]),
                &gpu.shader(include_str!("y.wgsl")),
            ),
            z: gpu.compute(
                "Tangent::Z",
                &gpu.pipeline_layout(&[
                    &TangentBuffer::layout(gpu),
                    &MipTexture3D::<Rgba8Unorm>::layout_write(gpu),
                ]),
                &gpu.shader(include_str!("z.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Tangent"),
            ..Default::default()
        });

        let n = frame.tangent().tangent().resolution().div_ceil(4);

        pass.set_bind_group(0, frame.tangent().binding(), &[]);

        pass.set_pipeline(&self.x);
        pass.set_bind_group(1, frame.occupancy().density().binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.y);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.z);
        pass.set_bind_group(1, frame.tangent().tangent().binding_write(), &[]);
        pass.dispatch_workgroups(n, n, n);
    }
}
