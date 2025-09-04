pub mod culling;
pub mod occlusion;
pub mod occupancy;
pub mod occupancy_alt;
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
        culling::LineCullingPipeline, occlusion::LineOcclusionPipeline,
        occupancy_alt::LineOccupancyAltPipeline, render::LineRenderPipeline,
        transform::LineTransformPipeline, vrc::VrcLineVoxelizationPipeline,
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

    vrc: VrcLineVoxelizationPipeline,
    occupancy_alt: LineOccupancyAltPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),

            occupancy: LineOccupancyPipeline::new(gpu),
            occlusion: LineOcclusionPipeline::new(gpu),
            culling: LineCullingPipeline::new(gpu),
            render: LineRenderPipeline::new(gpu),
            vrc: VrcLineVoxelizationPipeline::new(gpu),
            occupancy_alt: LineOccupancyAltPipeline::new(gpu),
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

        match settings.render {
            LineRenderMode::RayTracingAlt => {
                self.occupancy_alt
                    .dispatch(cmd, frame, environment, settings.voxelization, line)
            }
            LineRenderMode::QuantizedRayCasting => self.vrc.dispatch(cmd, frame, environment, line),
            _ => self
                .occupancy
                .dispatch(cmd, frame, environment, settings.voxelization, line),
        }

        self.culling.dispatch(cmd, frame, environment);
        self.occlusion.dispatch(cmd, frame, environment);

        self.render
            .dispatch(cmd, environment, frame, line, settings);
    }
}
