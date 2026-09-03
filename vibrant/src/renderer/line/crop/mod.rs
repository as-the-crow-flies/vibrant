use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{crop::CropBuffer, line::LineBuffer, tractography::Tractography, Asset},
    gpu::Gpu,
};

pub struct LineCropPipeline {
    crop: ComputePipeline,
    adjacency: ComputePipeline,
}

impl LineCropPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            crop: gpu.compute(
                "Crop",
                &gpu.pipeline_layout(&[
                    &LineBuffer::layout_crop(gpu),
                    &Tractography::layout(gpu),
                    &CropBuffer::layout(gpu),
                ]),
                &gpu.shader(include_str!("crop.wgsl")),
            ),
            adjacency: gpu.compute(
                "Adjacency",
                &gpu.pipeline_layout(&[&LineBuffer::layout_transform(gpu)]),
                &gpu.shader(include_str!("adjacency.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, asset: &Asset, line: &LineBuffer) {
        self.crop(cmd, asset, line);
        self.adjacency(cmd, line);
    }

    fn crop(&self, cmd: &mut CommandEncoder, asset: &Asset, line: &LineBuffer) {
        line.clear_length(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Crop"),
            ..Default::default()
        });

        pass.set_pipeline(&self.crop);
        pass.set_bind_group(0, line.binding_crop(), &[]);
        pass.set_bind_group(1, asset.tractography.binding(), &[]);
        pass.set_bind_group(2, asset.crop.binding(), &[]);
        pass.dispatch_workgroups(line.n_lines().div_ceil(32), 1, 1);
    }

    fn adjacency(&self, cmd: &mut CommandEncoder, line: &LineBuffer) {
        line.clear_offset(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Adjacency"),
            ..Default::default()
        });

        pass.set_pipeline(&self.adjacency);
        pass.set_bind_group(0, line.binding_transform(), &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
}
