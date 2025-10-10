pub mod raytracing;
pub mod volume;

use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::{LineDisplayMode, Settings},
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::{
            raytracing::RayTracingLineRenderPipeline, volume::VolumeLineRenderPipeline,
        },
    },
    surface::Frame,
};

pub struct LineRenderPipeline {
    ray: RayTracingLineRenderPipeline,
    volume: VolumeLineRenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            ray: RayTracingLineRenderPipeline::new(gpu),
            volume: VolumeLineRenderPipeline::new(gpu),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        line: &LineSet,
        settings: &Settings,
    ) {
        match settings.display {
            LineDisplayMode::Geometry => self.ray.render(cmd, frame, environment, settings, line),
            LineDisplayMode::Volume => self.volume.render(cmd, frame, environment),
        }
    }
}
