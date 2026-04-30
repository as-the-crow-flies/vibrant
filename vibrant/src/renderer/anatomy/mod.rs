pub mod gradient;
pub mod radiance;
pub mod trace;
pub mod transfer;

use wgpu::CommandEncoder;

use crate::{
    asset::Asset,
    controller::Controller,
    gpu::Gpu,
    renderer::{
        anatomy::{
            gradient::GradientPipeline, radiance::AnatomyRadiancePipeline,
            trace::AnatomyTracePipeline, transfer::AnatomyTransferPipeline,
        },
        environment::Environment,
    },
    surface::Surface,
};

pub struct AnatomyRenderer {
    transfer: AnatomyTransferPipeline,
    gradient: GradientPipeline,
    radiance: AnatomyRadiancePipeline,
    trace: AnatomyTracePipeline,
}

impl AnatomyRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: AnatomyTransferPipeline::new(gpu),
            gradient: GradientPipeline::new(gpu),
            radiance: AnatomyRadiancePipeline::new(gpu),
            trace: AnatomyTracePipeline::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        environment: &Environment,
        surface: &Surface,
        asset: &Asset,
    ) {
        if let (Some(volume), Some(radiance)) = (&asset.physical_volume, &asset.radiance) {
            if surface.changed() | asset.changed() | controller.changed() {
                self.transfer
                    .dispatch(cmd, &asset.volumes, &asset.masks, volume, &asset.crop);

                self.gradient.dispatch(cmd, volume);

                self.radiance
                    .dispatch(cmd, environment, &asset.hdri, volume, radiance);
            }

            self.trace.dispatch(
                cmd,
                environment,
                controller.viewport(),
                &asset.hdri,
                surface.frame(),
                volume,
                radiance,
            );
        }
    }
}
