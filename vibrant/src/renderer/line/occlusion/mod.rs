use wgpu::CommandEncoder;

use crate::{
    asset::line::LineBuffer,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::occlusion::{
            ambient::AmbientOcclusionPipeline, directional::DirectionalOcclusionPipeline,
        },
    },
};

pub mod ambient;
pub mod directional;

pub struct LineOcclusionPipeline {
    ambient: AmbientOcclusionPipeline,
    directional: DirectionalOcclusionPipeline,
}

impl LineOcclusionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            ambient: AmbientOcclusionPipeline::new(gpu),
            directional: DirectionalOcclusionPipeline::new(gpu),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, line: &LineBuffer, environment: &Environment) {
        self.ambient.render(cmd, line, environment);
        self.directional.render(cmd, line, environment);
    }
}
