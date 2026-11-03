pub mod render;
pub mod transfer;

use wgpu::CommandEncoder;

use crate::{
    asset::{volume::PhysicalVolume, Asset},
    controller::Controller,
    gpu::Gpu,
    renderer::volume::{render::GaussianVolumeRenderer, transfer::VolumeTransferPipeline},
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

    /// Clear + accumulate the visible volume fractions into the `r32uint`
    /// textures, then resolve them into the sampled textures + mip chain. The
    /// line deposit writes its own `line_extinction` texture separately.
    pub fn transfer(&self, cmd: &mut CommandEncoder, asset: &Asset, volume: &PhysicalVolume) {
        self.transfer
            .dispatch(cmd, &asset.volumes, &asset.masks, volume, &asset.crop);
    }

    pub fn render(
        &self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        asset: &Asset,
        frame: &Frame,
    ) {
        // In a lines-only scene the PhysicalVolume holds nothing but deposited
        // line density (used to build the cascade); tracing it would just wrap
        // every line in haze.
        if asset.volumes.is_empty() {
            return;
        }

        self.render.dispatch(gpu, cmd, asset, controller, frame);
    }
}
