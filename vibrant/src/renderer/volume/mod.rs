pub mod gradient;
pub mod render;
pub mod transfer;

use wgpu::CommandEncoder;

use crate::{
    asset::Asset,
    controller::Controller,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        volume::{
            gradient::GradientPipeline, render::VolumeRenderPipeline,
            transfer::VolumeTransferPipeline,
        },
    },
    surface::Surface,
};

pub struct VolumeRenderer {
    transfer: VolumeTransferPipeline,
    gradient: GradientPipeline,
    render: VolumeRenderPipeline,
}

impl VolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: VolumeTransferPipeline::new(gpu),
            gradient: GradientPipeline::new(gpu),
            render: VolumeRenderPipeline::new(gpu),
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
            let recompute = surface.changed() | asset.changed() | controller.lighting_changed();

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
                controller.radiance().lobes(),
            );
        }
    }
}
