pub mod render;
pub mod transfer;

use wgpu::CommandEncoder;

use crate::{
    asset::{volume::PhysicalVolume, Asset},
    controller::Controller,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        volume::{render::GaussianVolumeRenderer, transfer::VolumeTransferPipeline},
    },
    surface::Frame,
};

pub struct VolumeRenderer {
    transfer: VolumeTransferPipeline,
    render: GaussianVolumeRenderer,
}

impl VolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: VolumeTransferPipeline::new(gpu),
            render: GaussianVolumeRenderer::new(gpu),
        }
    }

    pub fn transfer(&self, cmd: &mut CommandEncoder, asset: &Asset, volume: &PhysicalVolume) {
        self.transfer
            .dispatch(cmd, &asset.volumes, &asset.masks, volume, &asset.crop);
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        environment: &Environment,
        asset: &Asset,
        frame: &Frame,
    ) {
        let (Some(volume), Some(radiance)) = (&asset.physical_volume, &asset.radiance) else {
            return;
        };

        self.render.dispatch(
            cmd,
            environment,
            &asset.hdri,
            frame,
            radiance,
            volume,
            controller.viewport(),
            controller.radiance().lobes(),
        );
    }
}
