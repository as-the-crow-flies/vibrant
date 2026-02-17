use std::ops::{Add, Div};

use wgpu::*;

use crate::{
    asset::{segmentation::VolumeSegmenationBuffer, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct AnatomyTransferPipeline {
    transfer: ComputePipeline,
    mipmap: ComputePipeline,
}

impl AnatomyTransferPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: gpu.compute(
                "AnatomyTransferPipeline",
                &gpu.pipeline_layout(&[
                    &VolumeSegmenationBuffer::layout(gpu, TextureSampleType::Uint),
                    &PhysicalVolume::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("transfer.wgsl")),
            ),
            mipmap: gpu.compute(
                "Occupancy::MipMap",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        segmentation: &VolumeSegmenationBuffer,
        volume: &PhysicalVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.transfer);
        pass.set_bind_group(0, segmentation.binding(), &[]);
        pass.set_bind_group(1, volume.binding_write(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);

        let mut n_workgroups = volume.size().add(3).div(4);

        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.mipmap);

        for binding in volume.bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

            n_workgroups = n_workgroups.add(1).div(2);
        }
    }
}
