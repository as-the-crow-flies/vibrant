use std::ops::{Add, Div};

use wgpu::*;

use crate::{
    asset::{
        segmentation::VolumeSegmenationBuffer, volume::PhysicalVolume,
        volume_fraction::VolumeFractionBuffer,
    },
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct AnatomyTransferPipeline {
    clear: ComputePipeline,
    fractions: ComputePipeline,
    segmentation: ComputePipeline,
}

impl AnatomyTransferPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            clear: gpu.compute(
                "AnatomyClearPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_write(gpu)]),
                &gpu.shader(include_str!("clear.wgsl")),
            ),
            fractions: gpu.compute(
                "AnatomyFractionsTransferPipeline",
                &gpu.pipeline_layout(&[
                    &VolumeFractionBuffer::layout(gpu),
                    &PhysicalVolume::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("fractions.wgsl")),
            ),
            segmentation: gpu.compute(
                "AnatomySegmentationTransferPipeline",
                &gpu.pipeline_layout(&[
                    &VolumeSegmenationBuffer::layout(gpu, TextureSampleType::Uint),
                    &PhysicalVolume::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("segmentation.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        fractions: &[VolumeFractionBuffer],
        segmentations: &[VolumeSegmenationBuffer],
        volume: &PhysicalVolume,
    ) {
        let n_workgroups = volume.size().add(3).div(4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.clear);
        pass.set_bind_group(0, volume.binding_write(), &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_bind_group(1, volume.binding_write(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);

        pass.set_pipeline(&self.fractions);
        for fraction in fractions.iter().filter(|x| x.settings().visible) {
            pass.set_bind_group(0, fraction.binding(), &[]);
            pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
        }

        pass.set_pipeline(&self.segmentation);
        for segmentation in segmentations {
            pass.set_bind_group(0, segmentation.binding(), &[]);
            pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
        }
    }
}
