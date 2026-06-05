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
            gradient::GradientPipeline, render::octahedral::OctahedralVolumeRenderer,
            transfer::AnatomyTransferPipeline,
        },
        environment::Environment,
    },
    surface::Surface,
};

pub struct AnatomyRenderer {
    transfer: AnatomyTransferPipeline,
    gradient: GradientPipeline,
    render: OctahedralVolumeRenderer,
}

impl AnatomyRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transfer: AnatomyTransferPipeline::new(gpu),
            gradient: GradientPipeline::new(gpu),
            render: OctahedralVolumeRenderer::new(gpu),
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
        if let (Some(frame), Some(volume), Some(radiance)) =
            (surface.frame(), &asset.physical_volume, &asset.radiance)
        {
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
                radiance,
                volume,
                controller.viewport(),
            );
        }
    }
}
