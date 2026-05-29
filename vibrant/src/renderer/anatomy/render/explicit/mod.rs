use egui::Rect;
use wgpu::CommandEncoder;

use crate::{
    asset::{hdri::HdriBuffer, radiance::RadianceVolume, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::{
        anatomy::render::explicit::{
            cascade::ExplicitCascadePipeline, trace::ExplicitTracePipeline,
        },
        environment::Environment,
    },
    surface::Frame,
};

pub mod cascade;
pub mod trace;

pub struct ExplicitRenderPipeline {
    cascade: ExplicitCascadePipeline,
    trace: ExplicitTracePipeline,
}

impl ExplicitRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            cascade: ExplicitCascadePipeline::new(gpu),
            trace: ExplicitTracePipeline::new(gpu),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        viewport: Rect,
        hdri: &HdriBuffer,
        frame: &Frame,
        volume: &PhysicalVolume,
        radiance: &RadianceVolume,
        recompute: bool,
    ) {
        if recompute {
            self.cascade
                .dispatch(cmd, environment, hdri, volume, radiance);
        }
        self.trace
            .dispatch(cmd, environment, viewport, hdri, frame, volume, radiance);
    }
}
