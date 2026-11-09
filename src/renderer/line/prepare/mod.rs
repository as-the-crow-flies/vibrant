use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{crop::CropBuffer, line::LineBuffer, tractography::Tractography, Asset},
    gpu::{Gpu, Lazy},
};

const WORKGROUP_SIZE: u32 = 256;
const MAX_WORKGROUPS: u32 = 65535;

/// Transforms and crops the raw lines into `LINE_VERTEX` / `LINE_INDEX`.
pub struct LinePreparePipeline {
    prepare: Lazy<ComputePipeline>,
}

impl LinePreparePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            prepare: gpu.lazy("LinePrepare", move |gpu| {
                gpu.compute(
                    "LinePrepare",
                    &gpu.pipeline_layout(&[
                        &LineBuffer::layout_prepare(gpu),
                        &Tractography::layout(gpu),
                        &CropBuffer::layout(gpu),
                    ]),
                    &gpu.shader("renderer/line/prepare/prepare.wgsl"),
                )
            }),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, asset: &Asset, line: &LineBuffer) {
        line.clear_length(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("LinePrepare"),
            ..Default::default()
        });

        pass.set_pipeline(&self.prepare);
        pass.set_bind_group(0, line.binding_prepare(), &[]);
        pass.set_bind_group(1, asset.tractography.binding(), &[]);
        pass.set_bind_group(2, asset.crop.binding(), &[]);
        pass.dispatch_workgroups(
            line.n_vertices()
                .div_ceil(WORKGROUP_SIZE)
                .min(MAX_WORKGROUPS),
            1,
            1,
        );
    }
}
