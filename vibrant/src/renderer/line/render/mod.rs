pub mod raytracing;
pub mod volume;

use wgpu::{BindGroup, CommandEncoder, TextureFormat};

use crate::{
    asset::line::LineBuffer,
    controller::settings::{LineDisplayMode, Settings},
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::{
            raytracing::RayTracingLineRenderPipeline, volume::VolumeLineRenderPipeline,
        },
    },
    surface::{color::ColorBuffer, Frame},
};

pub struct LineRenderPipeline {
    ray: RayTracingLineRenderPipeline,
    volume: VolumeLineRenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self::new_with_format(gpu, ColorBuffer::FORMAT)
    }

    pub fn new_with_format(gpu: &Gpu, format: TextureFormat) -> Self {
        Self {
            ray: RayTracingLineRenderPipeline::new_with_format(gpu, format),
            volume: VolumeLineRenderPipeline::new_with_format(gpu, format),
        }
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
    }

    pub fn dispatch_to_target(
        &self,
        cmd: &mut CommandEncoder,
        env_binding: &BindGroup,
        frame: &Frame,
        target: &ColorBuffer,
        line: &LineBuffer,
        settings: &Settings,
    ) {
        match settings.display {
            LineDisplayMode::Geometry => {
                self.ray.render_to(cmd, target, frame, env_binding, settings, line)
            }
            LineDisplayMode::Volume => {
                self.volume.render_to(cmd, target, frame, env_binding)
            }
        }
    }
}
