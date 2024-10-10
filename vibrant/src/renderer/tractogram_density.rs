use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline, PipelineLayoutDescriptor};

use crate::{
    asset::{density::Density, tractogram::Tractogram},
    gpu::Gpu,
};

use super::{constants::Constants, environment::Environment};

pub struct TractogramDensityRenderer {
    constants: Constants,
    clear_pipeline: ComputePipeline,
    rasterize_pipeline: ComputePipeline,
    copy_pipeline: ComputePipeline,
    mipmap_pipeline: ComputePipeline,
}

impl TractogramDensityRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        Self {
            constants: constants.clone(),
            clear_pipeline: gpu.compute(
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[&Density::layout_compute(gpu)],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    include_str!("wgsl/tractogram_density_clear.wgsl"),
                    Some(constants),
                ),
            ),
            rasterize_pipeline: gpu.compute(
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &Density::layout_compute(gpu),
                            &Tractogram::layout_full(gpu),
                            &Environment::layout(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    &(Environment::wgsl() + include_str!("wgsl/tractogram_density_rasterize.wgsl")),
                    Some(constants),
                ),
            ),
            copy_pipeline: gpu.compute(
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[&Density::layout_copy(gpu)],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    include_str!("wgsl/tractogram_density_copy.wgsl"),
                    Some(constants),
                ),
            ),
            mipmap_pipeline: gpu.compute(
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[&Density::layout_mipmap(gpu)],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    include_str!("wgsl/tractogram_density_mipmap.wgsl"),
                    Some(constants),
                ),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        tractogram: &Tractogram,
        density: &Density,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, density.binding_compute(), &[]);
        pass.set_bind_group(1, tractogram.binding_full(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);

        pass.set_pipeline(&self.clear_pipeline);

        let mut size = self.constants.num_workgroups_volume();
        pass.dispatch_workgroups(size, size, size);

        let count = tractogram.count().div_ceil(self.constants.workgroup_x);
        pass.set_pipeline(&self.rasterize_pipeline);
        pass.dispatch_workgroups(count, 1, 1);

        pass.set_pipeline(&self.copy_pipeline);
        pass.set_bind_group(0, density.binding_copy(), &[]);
        pass.dispatch_workgroups(size, size, size);

        pass.set_pipeline(&self.mipmap_pipeline);

        for binding in density.bindings_mipmap() {
            pass.set_bind_group(0, &binding, &[]);
            pass.dispatch_workgroups(size, size, size);

            size /= 2;
        }
    }
}
