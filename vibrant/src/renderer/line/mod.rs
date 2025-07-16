pub mod culling;
pub mod occlusion;
pub mod occupancy;
pub mod populate;
pub mod render;
pub mod transform;

use occupancy::LineOccupancyPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::line::{
        culling::LineCullingPipeline, occlusion::LineOcclusionPipeline, populate::PopulatePipeline,
        render::LineRenderPipeline, transform::LineTransformPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct LineRenderer {
    transform: LineTransformPipeline,
    occupancy: LineOccupancyPipeline,
    occlusion: LineOcclusionPipeline,
    culling: LineCullingPipeline,
    populate: PopulatePipeline,
    render: LineRenderPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),
            occupancy: LineOccupancyPipeline::new(gpu),
            occlusion: LineOcclusionPipeline::new(gpu),
            culling: LineCullingPipeline::new(gpu),
            populate: PopulatePipeline::new(gpu),
            render: LineRenderPipeline::new(gpu),
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
        self.transform.render(cmd, environment, line);
        self.occupancy
            .render(cmd, frame, environment, settings.voxelization, line);
        self.occlusion.render(cmd, frame, environment);
        self.culling.render(cmd, frame, environment);
        self.populate
            .render(cmd, frame, environment, settings.voxelization, line);

        self.render
            .render(cmd, environment, frame, line, settings.render);
    }
}
