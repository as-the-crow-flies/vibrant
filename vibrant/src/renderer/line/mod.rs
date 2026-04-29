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
    asset::Asset,
    controller::Controller,
    gpu::Gpu,
    renderer::line::{
        crop::LineCropPipeline, cull::LineCullPipeline, occlusion::LineOcclusionPipeline,
        populate::LinePopulatePipeline, post::PostProcessingPipeline, render::LineRenderPipeline,
        transform::LineTransformPipeline,
    },
    surface::Surface,
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

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        environment: &Environment,
        surface: &Surface,
        asset: &Asset,
    ) {
        if !controller.tractography().visible() {
            return;
        }

        let frame = surface.frame();

        if let (Some(line), Some(crop)) = (&asset.line, &asset.crop) {
            let changed = surface.changed() | asset.changed() | controller.changed();

            if changed {
                self.transform.dispatch(cmd, line, environment);

                self.crop.dispatch(cmd, line, environment, crop);

                self.occupancy
                    .dispatch(cmd, frame, environment, controller.settings(), line);

                self.cull.dispatch(cmd, frame, environment);

                self.populate
                    .dispatch(cmd, frame, environment, controller.settings(), line);
            }

            if changed || controller.lighting_changed() {
                self.occlusion.dispatch(cmd, frame, environment);
            }

            self.render.dispatch(
                cmd,
                frame,
                environment,
                controller.settings(),
                controller.viewport(),
                line,
            );

            self.post.dispatch(cmd, environment, frame);
        }
    }
}
