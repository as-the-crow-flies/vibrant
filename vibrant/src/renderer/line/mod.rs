pub mod culling;
pub mod occlusion;
pub mod occupancy;
pub mod render;
pub mod transform;
pub mod ui;

use occupancy::LineOccupancyPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    controller::settings::Settings,
    gpu::Gpu,
    renderer::line::{
        culling::LineCullingPipeline, occlusion::LineOcclusionPipeline, render::LineRenderPipeline,
        transform::LineTransformPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct LineRenderer {
    transform: LineTransformPipeline,
    occupancy: LineOccupancyPipeline,
    occlusion: LineOcclusionPipeline,
    culling: LineCullingPipeline,
    render: LineRenderPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),

            occupancy: LineOccupancyPipeline::new(gpu),
            occlusion: LineOcclusionPipeline::new(gpu),
            culling: LineCullingPipeline::new(gpu),
            render: LineRenderPipeline::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        line: &LineBuffer,
        transform: &TransformBuffer,
        settings: &Settings,
        needs_transform: bool,
    ) {
        if needs_transform {
            self.transform.dispatch(cmd, environment, line, transform);
        }

        self.occupancy
            .dispatch(cmd, frame, environment, settings, line);

        self.culling.dispatch(cmd, frame, environment);
        self.occlusion.dispatch(cmd, frame, environment);

        self.render
            .dispatch(cmd, environment, frame, line, settings);
    }
}
