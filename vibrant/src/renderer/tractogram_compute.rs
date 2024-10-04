use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline, PipelineLayoutDescriptor};

use crate::{
    asset::tractogram::Tractogram,
    gpu::Gpu,
    surface::{Frame, Surface},
};

use super::{camera::Camera, constants::Constants};

pub struct ComputeTractogramRenderer {
    num_workgroups: (u32, u32, u32),

    clear_pipeline: ComputePipeline,
    rasterize_pipeline: ComputePipeline,
}

impl ComputeTractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(
            include_str!("wgsl/tractogram_compute.wgsl"),
            Some(constants),
        );

        let layout = gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[
                    &Surface::create_visibility_buffer_layout(gpu),
                    &Camera::layout(gpu),
                    &Tractogram::layout_full(gpu),
                ],
                push_constant_ranges: &[],
            });

        Self {
            num_workgroups: constants.num_workgroups_xyz(),
            clear_pipeline: gpu.compute(&layout, &module, "clear"),
            rasterize_pipeline: gpu.compute(&layout, &module, "draw"),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        camera: &Camera,
        frame: &Frame,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, camera.binding(), &[]);
        pass.set_bind_group(2, tractogram.binding_full(), &[]);

        pass.set_pipeline(&self.clear_pipeline);

        let (x, y, z) = self.num_workgroups;
        pass.dispatch_workgroups(x, y, z);
    }
}
