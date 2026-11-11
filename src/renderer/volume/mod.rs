pub mod render;
pub mod transfer;

use wgpu::CommandEncoder;

use crate::{
    asset::Asset,
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

    /// After any fraction, mask or crop edit.
    pub fn transfer(&self, gpu: &Gpu, cmd: &mut CommandEncoder, asset: &Asset) {
        self.transfer.dispatch(gpu, cmd, asset);
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
