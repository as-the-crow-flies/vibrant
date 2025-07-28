pub mod ray;
pub mod volume;

use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::LineRenderMode,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::{ray::RayCastingLineRenderPipeline, volume::VolumeLineRenderPipeline},
    },
    surface::Frame,
};

pub struct LineRenderPipeline {
    ray: RayCastingLineRenderPipeline,
    volume: VolumeLineRenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            ray: RayCastingLineRenderPipeline::new(gpu),
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
            LineRenderMode::RayCasting => self.ray.render(cmd, frame, environment, line),
            LineRenderMode::Volume => self.volume.render(cmd, frame, environment),
        }
    }
}
