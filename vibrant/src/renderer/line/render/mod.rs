use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::LineRenderMode,
    gpu::Gpu,
    renderer::{environment::Environment, line::render::ray::RayCastingLineRenderPipeline},
    surface::Frame,
};

pub mod ray;

pub struct LineRenderPipeline {
    hybrid: RayCastingLineRenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            hybrid: RayCastingLineRenderPipeline::new(gpu),
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
            LineRenderMode::RayCasting => self.hybrid.render(cmd, frame, environment, line),
        }
    }
}
