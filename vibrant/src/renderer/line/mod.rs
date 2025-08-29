pub mod culling;
pub mod occlusion;
pub mod occupancy;
pub mod render;
pub mod segment;
pub mod transform;
pub mod ui;
pub mod vrc;

use occupancy::LineOccupancyPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::{LineRenderMode, Settings},
    gpu::Gpu,
    renderer::line::{
        culling::LineCullingPipeline, occlusion::LineOcclusionPipeline, render::LineRenderPipeline,
        segment::LineSegmentPipeline, transform::LineTransformPipeline, ui::LineUiPipeline,
        vrc::LineVrcVoxelizationPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct LineRenderer {
    transform: LineTransformPipeline,
    segment: LineSegmentPipeline,
    vrc: LineVrcVoxelizationPipeline,
    occupancy: LineOccupancyPipeline,
    occlusion: LineOcclusionPipeline,
    culling: LineCullingPipeline,
    render: LineRenderPipeline,
    ui: LineUiPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),
            segment: LineSegmentPipeline::new(gpu),
            vrc: LineVrcVoxelizationPipeline::new(gpu),
            occupancy: LineOccupancyPipeline::new(gpu),
            occlusion: LineOcclusionPipeline::new(gpu),
            culling: LineCullingPipeline::new(gpu),
            render: LineRenderPipeline::new(gpu),
            ui: LineUiPipeline::new(gpu),
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
        self.transform.dispatch(cmd, environment, line);
        // self.segment.dispatch(cmd, environment, line);

        if settings.render == LineRenderMode::QuantizedRayCasting {
            self.vrc.dispatch(cmd, frame, environment, line);
        } else {
            self.occupancy
                .dispatch(cmd, frame, environment, settings.voxelization, line);
        }

        self.culling.dispatch(cmd, frame, environment);
        self.occlusion.dispatch(cmd, frame, environment);

        self.render
            .dispatch(cmd, environment, frame, line, settings);

        // self.ui.dispatch(cmd, frame, environment);
    }
}
