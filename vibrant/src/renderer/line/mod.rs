pub mod crop;
pub mod cull;
pub mod occlusion;
pub mod occupancy;
pub mod populate;
pub mod post;
pub mod render;
pub mod transform;

use occupancy::LineOccupancyPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    controller::settings::Settings,
    gpu::Gpu,
    renderer::line::{
        crop::LineCropPipeline, cull::LineCullPipeline, occlusion::LineOcclusionPipeline,
        populate::LinePopulatePipeline, post::PostProcessingPipeline, render::LineRenderPipeline,
        transform::LineTransformPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct LineRenderer {
    transform: LineTransformPipeline,
    crop: LineCropPipeline,
    occupancy: LineOccupancyPipeline,
    cull: LineCullPipeline,
    occlusion: LineOcclusionPipeline,
    populate: LinePopulatePipeline,
    render: LineRenderPipeline,
    post: PostProcessingPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),
            crop: LineCropPipeline::new(gpu),
            occupancy: LineOccupancyPipeline::new(gpu),
            occlusion: LineOcclusionPipeline::new(gpu),
            cull: LineCullPipeline::new(gpu),
            populate: LinePopulatePipeline::new(gpu),
            render: LineRenderPipeline::new(gpu),
            post: PostProcessingPipeline::new(gpu),
        }
    }

    pub fn update(&self, gpu: &Gpu, settings: &Settings) {
        self.crop.update(gpu, settings);
        self.render.update(gpu, settings);
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
        _needs_update: bool,
    ) {
        if needs_transform {
            self.transform.dispatch(cmd, line, transform, environment);
        }

        self.crop.dispatch(cmd, line, environment, settings);

        self.occupancy
            .dispatch(cmd, frame, environment, settings, line);

        self.cull.dispatch(cmd, frame, environment);

        self.occlusion.dispatch(cmd, frame, environment);

        self.populate
            .dispatch(cmd, frame, environment, settings, line);

        self.render
            .dispatch(cmd, environment, frame, line, settings);

        self.post.dispatch(cmd, environment, frame, settings);
    }
}
