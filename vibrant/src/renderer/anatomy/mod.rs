pub mod gradient;
pub mod render;
pub mod transfer;

use wgpu::CommandEncoder;

use crate::{
    asset::Asset,
    controller::Controller,
    gpu::Gpu,
    renderer::{
        anatomy::{
            gradient::GradientPipeline, render::AnatomyVolumeRenderer,
            transfer::AnatomyTransferPipeline,
        },
        environment::Environment,
    },
    surface::Surface,
};

pub struct AnatomyRenderer {
    transfer: AnatomyTransferPipeline,
    gradient: GradientPipeline,
    render: AnatomyVolumeRenderer,
}

impl AnatomyRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: AnatomyTransferPipeline::new(gpu),
            gradient: GradientPipeline::new(gpu),
            render: AnatomyVolumeRenderer::new(gpu),
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
        if let (Some(frame), Some(volume)) = (surface.frame(), &asset.physical_volume) {
            let recompute = surface.changed() | asset.changed() | controller.changed();

            if recompute {
                self.transfer
                    .dispatch(cmd, &asset.volumes, &asset.masks, volume, &asset.crop);

                self.gradient.dispatch(cmd, volume);
            }

            self.render.dispatch(
                cmd,
                environment,
                &asset.hdri,
                frame,
                &asset.radiance,
                volume,
                controller.viewport(),
                recompute,
            );
        }
    }
}
