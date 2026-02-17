pub mod radiance;
pub mod trace;
pub mod transfer;

use wgpu::CommandEncoder;

use crate::{
    asset::{
        radiance::RadianceVolume, segmentation::VolumeSegmenationBuffer, volume::PhysicalVolume,
    },
    gpu::Gpu,
    renderer::{
        anatomy::{
            radiance::AnatomyRadiancePipeline, trace::AnatomyTracePipeline,
            transfer::AnatomyTransferPipeline,
        },
        environment::Environment,
    },
    surface::Frame,
};

pub struct AnatomyRenderer {
    transfer: AnatomyTransferPipeline,
    radiance: AnatomyRadiancePipeline,
    trace: AnatomyTracePipeline,
}

impl AnatomyRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: AnatomyTransferPipeline::new(gpu),
            radiance: AnatomyRadiancePipeline::new(gpu),
            trace: AnatomyTracePipeline::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        segmentation: &VolumeSegmenationBuffer,
        volume: &PhysicalVolume,
        radiance: &RadianceVolume,
    ) {
        self.transfer
            .dispatch(cmd, environment, segmentation, volume);

        self.trace.dispatch(cmd, environment, frame, volume);
    }
}
