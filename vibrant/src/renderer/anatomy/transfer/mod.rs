use std::ops::{Add, Div};

use wgpu::*;

use crate::{
    asset::{
        crop::CropBuffer, volume::PhysicalVolume, volume_fraction::VolumeFractionBuffer,
        volume_mask::VolumeMaskBuffer,
    },
    gpu::Gpu,
};

pub struct AnatomyTransferPipeline {
    clear: ComputePipeline,
    transfer: ComputePipeline,
    copy: ComputePipeline,
}

impl AnatomyTransferPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            clear: gpu.compute(
                "AnatomyClearPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_transfer(gpu)]),
                &gpu.shader(include_str!("clear.wgsl")),
            ),
            transfer: gpu.compute(
                "AnatomyTransferPipeline",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_transfer(gpu),
                    &VolumeFractionBuffer::layout(gpu),
                    &VolumeMaskBuffer::layout(gpu),
                    &CropBuffer::layout(gpu),
                ]),
                &gpu.shader(include_str!("transfer.wgsl")),
            ),
            copy: gpu.compute(
                "AnatomyCopyPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_copy(gpu)]),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        fractions: &[VolumeFractionBuffer],
        masks: &[VolumeMaskBuffer],
        volume: &PhysicalVolume,
        crop: &CropBuffer,
    ) {
        let n_workgroups = volume.size().add(3).div(4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, volume.binding_transfer(), &[]);

        pass.set_pipeline(&self.clear);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_bind_group(3, crop.binding(), &[]);

        pass.set_pipeline(&self.transfer);
        for fraction in fractions.iter().filter(|x| x.settings().visible) {
            pass.set_bind_group(1, fraction.binding(), &[]);
            pass.set_bind_group(2, masks[fraction.settings().mask].binding(), &[]);
            pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
        }

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, volume.binding_copy(), &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
    }
}
