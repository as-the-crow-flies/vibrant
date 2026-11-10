use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::{occupancy::OccupancyBuffer, LineBuffer},
        texture::{MipTexture3D, R32Float, R32Uint},
        tractography::Tractography,
        Asset,
    },
    controller::Controller,
    gpu::{Gpu, Lazy},
};

pub struct LineOccupancyPipeline {
    voxelize: Lazy<ComputePipeline>,
    copy: Lazy<ComputePipeline>,
    mipmap: Lazy<ComputePipeline>,
}

impl LineOccupancyPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            voxelize: gpu.lazy("Occupancy::Voxelize", move |gpu| {
                gpu.compute(
                    "Occupancy::Voxelize",
                    &gpu.pipeline_layout(&[
                        &OccupancyBuffer::layout_write(gpu),
                        &LineBuffer::layout_render(gpu),
                        &Tractography::layout(gpu),
                    ]),
                    &gpu.shader("renderer/line/occupancy/occupancy.wgsl"),
                )
            }),
            copy: gpu.lazy("Occupancy::Copy", move |gpu| {
                gpu.compute(
                    "Occupancy::Copy",
                    &gpu.pipeline_layout(&[
                        &OccupancyBuffer::layout_write(gpu),
                        &MipTexture3D::<R32Float>::layout_write(gpu),
                        &MipTexture3D::<R32Uint>::layout_write(gpu),
                        &Tractography::layout(gpu),
                    ]),
                    &gpu.shader("renderer/line/occupancy/copy.wgsl"),
                )
            }),
            mipmap: gpu.lazy("Occupancy::MipMap", move |gpu| {
                gpu.compute(
                    "Occupancy::MipMap",
                    &gpu.pipeline_layout(&[&MipTexture3D::<R32Float>::layout_mipmap(gpu)]),
                    &gpu.shader("renderer/line/occupancy/mipmap.wgsl"),
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

        line.occupancy().clear(cmd);
        line.clear_offset(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occupancy"),
            ..Default::default()
        });

        let n = line.occupancy().pyramid().resolution().div_ceil(4);

        pass.set_bind_group(0, line.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, line.binding_render(), &[]);
        pass.set_bind_group(2, asset.tractography.binding(), &[]);

        pass.set_pipeline(&self.voxelize);
        pass.dispatch_workgroups(settings.workgroups, 1, 1);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, line.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, line.occupancy().pyramid().binding_write(), &[]);
        pass.set_bind_group(2, line.occupancy().count().binding_write(), &[]);
        pass.set_bind_group(3, asset.tractography.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = line.occupancy().pyramid().resolution().div_ceil(4);

        for binding in line.occupancy().pyramid().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
