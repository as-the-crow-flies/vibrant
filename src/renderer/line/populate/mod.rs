use wgpu::{CommandEncoder, ComputePipeline};

use crate::{
    asset::{
        line::{voxel_index::VoxelIndexBuffer, LineBuffer},
        texture::{MipTexture3D, R32Uint},
        tractography::Tractography,
        Asset,
    },
    controller::Controller,
    gpu::{compute_pass, Gpu, Lazy},
};

pub struct LinePopulatePipeline {
    scan: Lazy<ComputePipeline>,
    populate: Lazy<ComputePipeline>,
}

impl LinePopulatePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            scan: gpu.lazy("Populate::Scan", move |gpu| {
                gpu.compute(
                    "Populate::Scan",
                    &gpu.pipeline_layout(&[
                        &VoxelIndexBuffer::layout_write(gpu),
                        &MipTexture3D::<R32Uint>::layout(gpu),
                    ]),
                    &gpu.shader("renderer/line/populate/scan.wgsl"),
                )
            }),
            populate: gpu.lazy("Populate::Populate", move |gpu| {
                gpu.compute(
                    "Populate::Populate",
                    &gpu.pipeline_layout(&[
                        &VoxelIndexBuffer::layout_write(gpu),
                        &LineBuffer::layout_render(gpu),
                        &Tractography::layout(gpu),
                    ]),
                    &gpu.shader("renderer/line/populate/populate.wgsl"),
                )
            }),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        asset: &Asset,
        controller: &Controller,
        line: &LineBuffer,
    ) {
        let settings = controller.settings();

        line.voxel_index().clear(cmd);
        line.clear_offset(cmd);

        let n = line.occupancy().resolution().div_ceil(4);

        let mut pass = compute_pass(cmd, "line/populate/scan");
        pass.set_pipeline(&self.scan);
        pass.set_bind_group(0, line.voxel_index().binding_write(), &[]);
        pass.set_bind_group(1, line.occupancy().count().binding(), &[]);
        pass.dispatch_workgroups(n, n, n);
        drop(pass);

        let mut pass = compute_pass(cmd, "line/populate/populate");
        pass.set_pipeline(&self.populate);
        pass.set_bind_group(0, line.voxel_index().binding_write(), &[]);
        pass.set_bind_group(1, line.binding_render(), &[]);
        pass.set_bind_group(2, asset.tractography.binding(), &[]);
        pass.dispatch_workgroups(settings.workgroups, 1, 1);
    }
}
