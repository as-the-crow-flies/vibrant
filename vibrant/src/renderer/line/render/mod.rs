pub mod raster;
pub mod ray;
pub mod volume;

use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::{LineRenderMode, Settings},
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::{
            raster::LineRasterizationPipeline, ray::RayCastingLineRenderPipeline,
            volume::VolumeLineRenderPipeline,
        },
    },
    surface::Frame,
};

pub struct LineRenderPipeline {
    raster: LineRasterizationPipeline,
    ray: RayCastingLineRenderPipeline,
    volume: VolumeLineRenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            raster: LineRasterizationPipeline::new(gpu),
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
        settings: &Settings,
    ) {
        match settings.render {
            LineRenderMode::Rasterization => {
                self.raster.render(cmd, frame, environment, line, settings)
            }
            LineRenderMode::RayCasting => self.ray.render(cmd, frame, environment, line),
            LineRenderMode::Volume => self.volume.render(cmd, frame, environment),
        }
    }
}
