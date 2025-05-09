use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        filter::Filter,
        scalar::{ScalarTexture2D, ScalarTexture3D},
        tractogram::Tractogram,
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{slice::SliceBuffer, SurfaceBuffer},
};

pub struct TractogramSlicePipeline {
    slice: ComputePipeline,
    mipmap: ComputePipeline,
    cull: ComputePipeline,
}

impl TractogramSlicePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            slice: gpu.compute(
                "Slice::Slice",
                &gpu.pipeline_layout(&[
                    &ScalarTexture3D::layout(gpu),
                    &ScalarTexture2D::layout_write(gpu),
                    &SliceBuffer::layout(gpu, false),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("slice.wgsl")),
            ),
            mipmap: gpu.compute(
                "Slice::Mipmap",
                &gpu.pipeline_layout(&[&ScalarTexture2D::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
            cull: gpu.compute(
                "Slice::Cull",
                &gpu.pipeline_layout(&[
                    &ScalarTexture2D::layout(gpu),
                    &Tractogram::layout(gpu),
                    &Environment::layout(gpu),
                    &Filter::layout_write(gpu),
                ]),
                &gpu.shader(include_str!("cull.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        buffer: &SurfaceBuffer,
        environment: &Environment,
        tractogram: &Tractogram,
    ) {
        tractogram.filter_culling().clear(cmd);
        buffer.slice().clear(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        let mut x = buffer.slice().hiz().width().div_ceil(8);
        let mut y = buffer.slice().hiz().height().div_ceil(8);

        pass.set_pipeline(&self.slice);
        pass.set_bind_group(0, buffer.density().volume().binding(), &[]);
        pass.set_bind_group(1, buffer.slice().hiz().binding_write(), &[]);
        pass.set_bind_group(2, buffer.slice().binding(false), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(x, y, 1);

        pass.set_pipeline(&self.mipmap);
        pass.set_bind_group(1, environment.binding(), &[]);
        for binding in buffer.slice().hiz().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(x, y, 1);

            x /= 2;
            y /= 2;
        }

        pass.set_pipeline(&self.cull);
        pass.set_bind_group(0, buffer.slice().hiz().binding(), &[]);
        pass.set_bind_group(1, tractogram.binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, tractogram.filter_culling().binding_write(), &[]);
        pass.dispatch_workgroups(tractogram.vertex_count().div_ceil(32 * 1024), 1, 1);
    }
}
