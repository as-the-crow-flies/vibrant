use std::ops::{Add, Div};

use wgpu::*;

use crate::{
    asset::{
        volume::PhysicalVolume, volume_fraction::VolumeFractionBuffer,
        volume_mask::VolumeMaskBuffer,
    },
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct AnatomyTransferPipeline {
    clear: ComputePipeline,
    transfer: ComputePipeline,
}

impl AnatomyTransferPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            clear: gpu.compute(
                "AnatomyClearPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_write(gpu)]),
                &gpu.shader(include_str!("clear.wgsl")),
            ),
            transfer: gpu.compute(
                "AnatomyTransferPipeline",
                &gpu.pipeline_layout(&[
                    &VolumeFractionBuffer::layout(gpu),
                    &VolumeMaskBuffer::layout(gpu),
                    &PhysicalVolume::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("transfer.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        fractions: &[VolumeFractionBuffer],
        mask: &VolumeMaskBuffer,
        volume: &PhysicalVolume,
    ) {
        let n_workgroups = volume.size().add(3).div(4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, volume.binding_write(), &[]);
        pass.set_bind_group(1, mask.binding(), &[]);
        pass.set_bind_group(2, volume.binding_write(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);

        pass.set_pipeline(&self.clear);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.transfer);
        for fraction in fractions.iter().filter(|x| x.settings().visible) {
            pass.set_bind_group(0, fraction.binding(), &[]);
            pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
        }
    }
}
