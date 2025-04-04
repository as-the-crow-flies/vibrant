use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        filter::Filter,
        scalar::{ScalarTexture2D, ScalarTexture3D},
        tractogram::Tractogram,
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::SurfaceBuffer,
};

pub struct TractogramOcclusionPipeline {
    copy: ComputePipeline,
    accumulate: ComputePipeline,
    erode: ComputePipeline,
    mipmap: ComputePipeline,
    cull: ComputePipeline,
}

impl TractogramOcclusionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            copy: gpu.compute(
                "Occlusion::Copy",
                &gpu.pipeline_layout(&[
                    &ScalarTexture3D::layout_write(gpu),
                    &ScalarTexture3D::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(&(Environment::wgsl() + include_str!("copy.wgsl"))),
                "main",
            ),
            accumulate: gpu.compute(
                "Occlusion::Accumulate",
                &gpu.pipeline_layout(&[
                    &ScalarTexture3D::layout_write(gpu),
                    &ScalarTexture2D::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(&(Environment::wgsl() + include_str!("accumulate.wgsl"))),
                "main",
            ),
            erode: gpu.compute(
                "Occlusion::Erode",
                &gpu.pipeline_layout(&[
                    &ScalarTexture2D::layout(gpu),
                    &ScalarTexture2D::layout_write(gpu),
                ]),
                &gpu.shader(include_str!("erode.wgsl")),
                "main",
            ),
            mipmap: gpu.compute(
                "Occlusion::Mipmap",
                &gpu.pipeline_layout(&[&ScalarTexture2D::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl")),
                "main",
            ),
            cull: gpu.compute(
                "Occlusion::Cull",
                &gpu.pipeline_layout(&[
                    &ScalarTexture2D::layout(gpu),
                    &Tractogram::layout(gpu),
                    &Environment::layout(gpu),
                    &Filter::layout_write(gpu),
                ]),
                &gpu.shader(&(Environment::wgsl() + include_str!("cull.wgsl"))),
                "main",
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

        let density = buffer.density().texture();
        let occlusion = buffer.occlusion();

        let mut x = occlusion.volume().width().div_ceil(8);
        let mut y = occlusion.volume().height().div_ceil(8);
        let z = occlusion.volume().depth().div_ceil(8);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occlusion"),
            ..Default::default()
        });

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, occlusion.volume().binding_write(), &[]);
        pass.set_bind_group(1, density.binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(x, y, z);

        pass.set_pipeline(&self.accumulate);
        pass.set_bind_group(1, occlusion.threshold().binding_write(), &[]);
        pass.dispatch_workgroups(x, y, 1);

        pass.set_pipeline(&self.erode);
        pass.set_bind_group(0, occlusion.threshold().binding(), &[]);
        pass.set_bind_group(1, occlusion.hiz().binding_write(), &[]);
        pass.dispatch_workgroups(x, y, 1);

        pass.set_pipeline(&self.mipmap);
        for binding in occlusion.hiz().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(x, y, 1);

            x /= 2;
            y /= 2;
        }

        pass.set_pipeline(&self.cull);
        pass.set_bind_group(0, occlusion.hiz().binding(), &[]);
        pass.set_bind_group(1, tractogram.binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, tractogram.filter_culling().binding_write(), &[]);
        pass.dispatch_workgroups(tractogram.vertex_count().div_ceil(32 * 1024), 1, 1);
    }
}
