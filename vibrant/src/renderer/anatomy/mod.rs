pub mod gaussian;
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
            gaussian::GaussianPipeline, gradient::GradientPipeline,
            radiance::AnatomyRadiancePipeline, trace::AnatomyTracePipeline,
            transfer::AnatomyTransferPipeline,
        },
        environment::Environment,
    },
    surface::Frame,
};

pub struct AnatomyRenderer {
    transfer: AnatomyTransferPipeline,
    gaussian: GaussianPipeline,
    gradient: GradientPipeline,
    radiance: AnatomyRadiancePipeline,
    trace: AnatomyTracePipeline,
}

impl AnatomyRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: AnatomyTransferPipeline::new(gpu),
            gaussian: GaussianPipeline::new(gpu),
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
        frame: &Frame,
        asset: &Asset,
    ) {
        if let (Some(volume), Some(radiance)) = (&asset.physical_volume, &asset.radiance) {
            if controller.volumes().changed() || controller.segmentations().changed() {
                self.transfer.dispatch(
                    cmd,
                    environment,
                    &asset.volume_fractions,
                    &asset.segmentations,
                    volume,
                );

                self.gaussian.dispatch(
                    cmd,
                    volume.binding_absorption(),
                    volume.binding_tmp(),
                    volume.size(),
                );
                self.gaussian.dispatch(
                    cmd,
                    volume.binding_scattering(),
                    volume.binding_tmp(),
                    volume.size(),
                );
                self.gaussian.dispatch(
                    cmd,
                    volume.binding_extinction(),
                    volume.binding_tmp(),
                    volume.size(),
                );

                self.gradient.dispatch(cmd, volume);

                self.radiance.dispatch(cmd, environment, volume, radiance);
            }

            self.trace
                .dispatch(cmd, environment, frame, volume, radiance);
        }
    }
}
