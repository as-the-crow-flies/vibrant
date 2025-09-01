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
        transform::LineTransformPipeline, vrc::VrcLineVoxelizationPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct LineRenderer {
    transform: LineTransformPipeline,
    vrc: VrcLineVoxelizationPipeline,
    occupancy: LineOccupancyPipeline,
    occlusion: LineOcclusionPipeline,
    culling: LineCullingPipeline,
    render: LineRenderPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),
            vrc: VrcLineVoxelizationPipeline::new(gpu),
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
        line: &LineSet,
        settings: &Settings,
    ) {
        self.transform.dispatch(cmd, environment, line);

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
    }
}
