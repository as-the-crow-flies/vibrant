use wgpu::CommandEncoder;

use crate::{
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::occlusion::{
            ambient::AmbientOcclusionPipeline, directional::DirectionalOcclusionPipeline,
        },
    },
    surface::Frame,
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

    pub fn dispatch(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        self.ambient.render(cmd, frame, environment);
        self.directional.render(cmd, frame, environment);
    }
}
