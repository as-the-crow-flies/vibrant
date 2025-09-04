pub mod rasterization;
pub mod raytracing;
pub mod raytracing_alt;
pub mod volume;
pub mod vrc;

use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::{LineDisplayMode, LineRenderMode, Settings},
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::{
            rasterization::LineRasterizationPipeline, raytracing::RayTracingLineRenderPipeline,
            raytracing_alt::RayTracingAltLineRenderPipeline, volume::VolumeLineRenderPipeline,
            vrc::VrcLineRenderPipeline,
        },
    },
    surface::Frame,
};

pub struct LineRenderPipeline {
    raster: LineRasterizationPipeline,
    ray: RayTracingLineRenderPipeline,
    ray_alt: RayTracingAltLineRenderPipeline,
    volume: VolumeLineRenderPipeline,
    vrc: VrcLineRenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            raster: LineRasterizationPipeline::new(gpu),
            ray: RayTracingLineRenderPipeline::new(gpu),
            ray_alt: RayTracingAltLineRenderPipeline::new(gpu),
            volume: VolumeLineRenderPipeline::new(gpu),
            vrc: VrcLineRenderPipeline::new(gpu),
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
            LineDisplayMode::Geometry => match settings.render {
                LineRenderMode::Rasterization => {
                    self.raster.render(cmd, frame, environment, line, settings)
                }
                LineRenderMode::RayTracing => {
                    self.ray.render(cmd, frame, environment, settings, line)
                }
                LineRenderMode::RayTracingAlt => {
                    self.ray_alt.render(cmd, frame, environment, settings, line)
                }
                LineRenderMode::QuantizedRayCasting => {
                    self.vrc.render(cmd, frame, environment, settings, line)
                }
            },
            LineDisplayMode::Volume => self.volume.render(cmd, frame, environment),
        }
    }
}
