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
    surface::Frame,
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
        frame: &Frame,
        asset: &Asset,
    ) {
        if let (Some(volume), Some(mask), Some(radiance), Some(hdri)) = (
            &asset.physical_volume,
            &asset.mask,
            &asset.radiance,
            &asset.hdri,
        ) {
            let data_changed = controller.volumes().changed()
                | controller.mask().changed()
                | controller.crop().changed();
            let lighting_changed = data_changed | controller.hdri().changed();

            if data_changed {
                self.transfer
                    .dispatch(cmd, environment, &asset.volumes, mask, volume);

                self.gradient.dispatch(cmd, volume);
            }

            if lighting_changed {
                self.radiance
                    .dispatch(cmd, environment, hdri, volume, radiance);
            }

            self.trace
                .dispatch(cmd, environment, hdri, frame, volume, radiance);
        }
    }
}
