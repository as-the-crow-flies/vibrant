use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::{voxel_index::VoxelIndexBuffer, LineBuffer},
        texture::{MipTexture3D, R32Uint},
        tractography::Tractography,
        Asset,
    },
    controller::{settings::LineVoxelizationMode, Controller},
    gpu::{Gpu, Lazy},
};

pub struct LinePopulatePipeline {
    scan: Lazy<ComputePipeline>,
    populate_tube: Lazy<ComputePipeline>,
    populate_line: Lazy<ComputePipeline>,
    populate_box: Lazy<ComputePipeline>,
}

impl LinePopulatePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = |gpu: &Gpu| {
            gpu.pipeline_layout(&[
                &VoxelIndexBuffer::layout_write(gpu),
                &LineBuffer::layout_render(gpu),
                &Tractography::layout(gpu),
            ])
        };

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
            populate_tube: gpu.lazy("Populate::Populate::Tube", move |gpu| {
                gpu.compute(
                    "Populate::Populate::Tube",
                    &layout(gpu),
                    &gpu.shader("renderer/line/populate/populate_tube.wgsl"),
                )
            }),
            populate_line: gpu.lazy("Populate::Populate::Line", move |gpu| {
                gpu.compute(
                    "Populate::Populate::Line",
                    &layout(gpu),
                    &gpu.shader("renderer/line/populate/populate_line.wgsl"),
                )
            }),
            populate_box: gpu.lazy("Populate::Populate::Box", move |gpu| {
                gpu.compute(
                    "Populate::Populate::Box",
                    &layout(gpu),
                    &gpu.shader("renderer/line/populate/populate_box.wgsl"),
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

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Populate"),
            ..Default::default()
        });

        let n = line.occupancy().resolution().div_ceil(4);

        pass.set_pipeline(&self.scan);
        pass.set_bind_group(0, line.voxel_index().binding_write(), &[]);
        pass.set_bind_group(1, line.occupancy().count().binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(match settings.voxelization {
            LineVoxelizationMode::Line => &self.populate_line,
            LineVoxelizationMode::Box => &self.populate_box,
            LineVoxelizationMode::Tube => &self.populate_tube,
        });

        pass.set_bind_group(0, line.voxel_index().binding_write(), &[]);
        pass.set_bind_group(1, line.binding_render(), &[]);
        pass.set_bind_group(2, asset.tractography.binding(), &[]);
        pass.dispatch_workgroups(settings.workgroups, 1, 1);
    }
}
