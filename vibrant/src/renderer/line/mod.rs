pub mod aa;
pub mod crop;
pub mod cull;
pub mod foveated;
pub mod occlusion;
pub mod occupancy;
pub mod populate;
pub mod post;
pub mod render;
pub mod transform;

use occupancy::LineOccupancyPipeline;
use wgpu::CommandEncoder;

use aa::AntiAliasingPipeline;
use foveated::FoveatedCompositePipeline;

use crate::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    controller::settings::Settings,
    gpu::Gpu,
    renderer::line::{
        crop::LineCropPipeline, cull::LineCullPipeline, occlusion::LineOcclusionPipeline,
        populate::LinePopulatePipeline, post::PostProcessingPipeline,
        render::LineRenderPipeline, transform::LineTransformPipeline,
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
    // Anti-aliasing pass: runs after post-processing, writes to frame.aa.
    aa: AntiAliasingPipeline,
    // Foveated rendering composite pass
    foveated: FoveatedCompositePipeline,
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
            aa: AntiAliasingPipeline::new(gpu),
            foveated: FoveatedCompositePipeline::new(gpu),
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

        if settings.foveated {
            // Pass 1: peripheral (low-res) → frame.foveated_peripheral
            if let Some(peripheral_buf) = frame.foveated_peripheral() {
                self.render.dispatch_to_target(
                    cmd,
                    environment.peripheral_binding(),
                    frame,
                    peripheral_buf,
                    line,
                    settings,
                );
            }

            // Pass 2: focus (high-res, sub-frustum) → frame.foveated_focus
            if let Some(focus_buf) = frame.foveated_focus() {
                self.render.dispatch_to_target(
                    cmd,
                    environment.focus_binding(),
                    frame,
                    focus_buf,
                    line,
                    settings,
                );
            }

            // Pass 3: composite peripheral + focus → frame.color (normal resolution)
            self.foveated.dispatch(cmd, environment, frame);
        } else {
            self.render
                .dispatch(cmd, environment, frame, line, settings);
        }

        self.post.dispatch(cmd, environment, frame, settings);

        self.aa.dispatch(cmd, environment, frame, settings);
    }
}
