use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline, PipelineLayoutDescriptor};

use crate::{
    asset::{density::Density, tractogram::Tractogram},
    gpu::Gpu,
    surface::Frame,
};

use super::{camera::Camera, constants::Constants};

pub struct TractogramDensityRenderer {
    constants: Constants,
    clear_pipeline: ComputePipeline,
    rasterize_pipeline: ComputePipeline,
    copy_pipeline: ComputePipeline,
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
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    include_str!("wgsl/tractogram_density_rasterize.wgsl"),
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
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        camera: &Camera,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, density.binding_compute(), &[]);
        pass.set_bind_group(1, tractogram.binding_full(), &[]);

        pass.set_pipeline(&self.clear_pipeline);

        let (x, y, z) = self.constants.num_workgroups_volume();
        pass.dispatch_workgroups(x, y, z);

        let count = tractogram.count().div_ceil(self.constants.workgroup_x);

        pass.set_pipeline(&self.rasterize_pipeline);
        pass.dispatch_workgroups(count, 1, 1);

        pass.set_pipeline(&self.copy_pipeline);
        pass.set_bind_group(0, density.binding_copy(), &[]);
        pass.dispatch_workgroups(x, y, z);
    }
}
