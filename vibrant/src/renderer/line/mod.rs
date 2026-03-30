pub mod aa;
pub mod crop;
pub mod cull;
pub mod occlusion;
pub mod occupancy;
pub mod populate;
pub mod post;
pub mod render;
pub mod transform;

use occupancy::LineOccupancyPipeline;
use wgpu::{CommandEncoder, TextureFormat};

use aa::AntiAliasingPipeline;

use crate::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    controller::settings::Settings,
    gpu::Gpu,
    renderer::line::{
        crop::LineCropPipeline, cull::LineCullPipeline, occlusion::LineOcclusionPipeline,
        populate::LinePopulatePipeline, post::PostProcessingPipeline, render::LineRenderPipeline,
        transform::LineTransformPipeline,
    },
    surface::color::ColorBuffer,
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
    // Anti-aliasing pass: runs after post-processing, writes to frame.aa.
    aa: AntiAliasingPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self::new_with_format(gpu, ColorBuffer::FORMAT)
    }

    pub fn new_with_format(gpu: &Gpu, format: TextureFormat) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),
            crop: LineCropPipeline::new(gpu),
            occupancy: LineOccupancyPipeline::new(gpu),
            occlusion: LineOcclusionPipeline::new(gpu),
            cull: LineCullPipeline::new(gpu),
            populate: LinePopulatePipeline::new(gpu),
            render: LineRenderPipeline::new_with_format(gpu, format),
            post: PostProcessingPipeline::new_with_format(gpu, format),
            aa: AntiAliasingPipeline::new_with_format(gpu, format),
        }
    }

    pub fn reset_taa_history(&mut self) {
        self.aa.reset_taa_history();
    }

    pub fn render(
        &mut self,
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

        self.crop.dispatch(cmd, line, environment);

        self.occupancy
            .dispatch(cmd, frame, environment, settings, line);

        self.cull.dispatch(cmd, frame, environment);

        self.occlusion.dispatch(cmd, frame, environment);

        self.populate
            .dispatch(cmd, frame, environment, settings, line);

        self.render
            .dispatch(cmd, environment, frame, line, settings);

        self.post.dispatch(cmd, environment, frame, settings);

        self.aa.dispatch(cmd, environment, frame, settings);
    }
}
