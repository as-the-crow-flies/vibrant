pub mod highlight;
pub mod raytracing;
pub mod volume;

use wgpu::CommandEncoder;

use crate::{
    asset::line::LineBuffer,
    controller::settings::{LineDisplayMode, Settings},
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::{
            highlight::VolumeHighlightPipeline, raytracing::RayTracingLineRenderPipeline,
            volume::VolumeLineRenderPipeline,
        },
    },
    surface::Frame,
};

pub struct LineRenderPipeline {
    ray: RayTracingLineRenderPipeline,
    volume: VolumeLineRenderPipeline,
    highlight: VolumeHighlightPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            ray: RayTracingLineRenderPipeline::new(gpu),
            volume: VolumeLineRenderPipeline::new(gpu),
            highlight: VolumeHighlightPipeline::new(gpu),
        }
    }

    pub fn update(&self, gpu: &Gpu, settings: &Settings) {
        self.highlight.update(gpu, settings);
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        line: &LineBuffer,
        settings: &Settings,
    ) {
        match settings.display {
            LineDisplayMode::Geometry => self.ray.render(cmd, frame, environment, settings, line),
            LineDisplayMode::Volume => self.volume.render(cmd, frame, environment),
        }
        self.highlight.render(cmd, environment, frame, settings);
    }
}
