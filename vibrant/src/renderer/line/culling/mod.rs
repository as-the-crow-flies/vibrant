use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::texture::{MipTexture3D, R32Float},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{culling::CullingBuffer, occupancy::OccupancyBuffer, Frame},
};

pub struct LineCullingPipeline {
    erode: ComputePipeline,
    culling: ComputePipeline,
    mipmap: ComputePipeline,
}

impl LineCullingPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            erode: gpu.compute(
                "Culling::Erode",
                &gpu.pipeline_layout(&[
                    &OccupancyBuffer::layout_read(gpu),
                    &CullingBuffer::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("erode.wgsl")),
            ),
            culling: gpu.compute(
                "Culling::Culling",
                &gpu.pipeline_layout(&[
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &CullingBuffer::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("culling.wgsl")),
            ),
            mipmap: gpu.compute(
                "Culling::Mipmap",
                &gpu.pipeline_layout(&[&MipTexture3D::<R32Float>::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Culling"),
            ..Default::default()
        });

        let n = frame.occupancy().resolution().div_ceil(4);

        pass.set_pipeline(&self.erode);
        pass.set_bind_group(0, frame.occupancy().binding(), &[]);
        pass.set_bind_group(1, frame.culling().binding_write(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.culling);
        pass.set_bind_group(0, frame.culling().pyramid().binding_write(), &[]);
        pass.set_bind_group(1, frame.occupancy().pyramid().binding_write(), &[]);
        pass.set_bind_group(2, frame.culling().binding_write(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = frame.culling().pyramid().resolution().div_ceil(8);
        for binding in frame.culling().pyramid().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
