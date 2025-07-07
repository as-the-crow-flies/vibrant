use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline, PipelineLayoutDescriptor};

use crate::{
    asset::{
        scalar::{R16Uint, R8Unorm, ScalarTexture3D},
        line::LineSet,
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{density::Density, Frame},
};

pub struct DensityPipeline {
    voxelize: ComputePipeline,
    copy: ComputePipeline,
    mipmap: ComputePipeline,
}

impl DensityPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        Self {
            voxelize: gpu.compute(
                "Density::Voxelize",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &Density::layout(gpu),
                            &ScalarTexture3D::<R8Unorm>::layout(gpu),
                            &LineSet::layout(gpu, true),
                            &Environment::layout(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    &(include_str!("../../wgsl/voxelize.wgsl").to_owned()
                        + include_str!("voxelize.wgsl")),
                ),
            ),
            copy: gpu.compute(
                "Density::Copy",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &Density::layout(gpu),
                            &ScalarTexture3D::<R8Unorm>::layout_write(gpu),
                            &ScalarTexture3D::<R16Uint>::layout_write(gpu),
                            &Environment::layout(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
            mipmap: gpu.compute(
                "Density::MipMap",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[&ScalarTexture3D::<R8Unorm>::layout_mipmap(gpu)],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        tractogram: &LineSet,
    ) {
        frame.density().clear(cmd);
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Density"),
            ..Default::default()
        });

        let n = frame.density().texture().size().div_ceil(8);

        pass.set_bind_group(0, frame.density().binding(), &[]);
        pass.set_bind_group(1, frame.density().texture().binding(), &[]);
        pass.set_bind_group(2, tractogram.binding(true), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);

        pass.set_pipeline(&self.voxelize);
        pass.dispatch_workgroups(64, 1, 1);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, frame.density().binding(), &[]);
        pass.set_bind_group(1, frame.density().texture().binding_write(), &[]);
        pass.set_bind_group(2, frame.density().count().binding_write(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = frame.density().texture().size().div_ceil(8);

        for binding in frame.density().texture().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
