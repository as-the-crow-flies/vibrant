pub mod density;
pub mod occlusion;
pub mod occupancy;
pub mod populate;
pub mod render;
pub mod shading;
pub mod transform;

use density::DensityPipeline;
use shading::volume::VolumeRenderPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::{Settings, ShadingSetting},
    gpu::Gpu,
    renderer::tractogram::{
        occlusion::OcclusionPipeline, occupancy::OccupancyPipeline, populate::PopulatePipeline,
        render::TractogramRenderPipeline, transform::TransformPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    transform: TransformPipeline,
    density: DensityPipeline,
    occlusion: OcclusionPipeline,
    occupancy: OccupancyPipeline,
    populate: PopulatePipeline,
    volume: VolumeRenderPipeline,
    render: TractogramRenderPipeline,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: TransformPipeline::new(gpu),
            density: DensityPipeline::new(gpu),
            occlusion: OcclusionPipeline::new(gpu),
            occupancy: OccupancyPipeline::new(gpu),
            populate: PopulatePipeline::new(gpu),
            volume: VolumeRenderPipeline::new(gpu),
            render: TractogramRenderPipeline::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &LineSet,
        settings: &Settings,
        needs_preprocess: bool,
    ) {
        if needs_preprocess {
            self.transform.render(cmd, tractogram);
        }

        self.density.render(cmd, frame, environment, tractogram);
        self.occlusion.render(cmd, frame, environment);
        self.occupancy.render(cmd, frame, environment);
        self.populate.render(cmd, frame, environment, tractogram);

        match settings.shading {
            ShadingSetting::Render => self.render.render(cmd, frame, environment, tractogram),
            ShadingSetting::Density => {
                self.volume
                    .render(cmd, frame, frame.density().texture(), environment)
            }
            ShadingSetting::Occlusion => {
                self.volume
                    .render(cmd, frame, frame.occlusion().texture(), environment)
            }
            ShadingSetting::Occupancy => {
                self.volume
                    .render(cmd, frame, frame.occupancy().texture(), environment)
            }
        }
    }
}
