use egui::Rect;
use wgpu::CommandEncoder;

use crate::{
    asset::{hdri::HdriBuffer, radiance::RadianceBuffer, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::{
        anatomy::render::{gaussian::GaussianVolumeRenderer, linear::LinearVolumeRenderer},
        environment::Environment,
    },
    surface::Frame,
};

pub mod gaussian;
pub mod linear;

pub struct AnatomyVolumeRenderer {
    linear: LinearVolumeRenderer,
    gaussian: GaussianVolumeRenderer,
}

impl AnatomyVolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            linear: LinearVolumeRenderer::new(gpu),
            gaussian: GaussianVolumeRenderer::new(gpu),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        radiance: &RadianceBuffer,
        volume: &PhysicalVolume,
        viewport: Rect,
        recompute: bool,
    ) {
        match radiance {
            RadianceBuffer::None => {}
            RadianceBuffer::Linear(radiance) => self.linear.dispatch(
                cmd,
                environment,
                &hdri,
                frame,
                radiance,
                volume,
                viewport,
                recompute,
            ),
            RadianceBuffer::Gaussian(radiance) => self.gaussian.dispatch(
                cmd,
                environment,
                &hdri,
                frame,
                radiance,
                volume,
                viewport,
                recompute,
            ),
        }
    }
}
