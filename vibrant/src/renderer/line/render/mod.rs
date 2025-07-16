use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::LineRenderMode,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::{hybrid::HybridLineRenderPipeline, volume::VolumeLineRenderPipeline},
    },
    surface::Frame,
};

pub mod hybrid;
pub mod volume;

pub struct LineRenderPipeline {
    hybrid: HybridLineRenderPipeline,
    volume: VolumeLineRenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            hybrid: HybridLineRenderPipeline::new(gpu),
            volume: VolumeLineRenderPipeline::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        line: &LineSet,
        mode: LineRenderMode,
    ) {
        match mode {
            LineRenderMode::Hybrid => self.hybrid.render(cmd, frame, environment, line),
            LineRenderMode::Volume => self.volume.render(cmd, frame, environment),
        }
    }
}
