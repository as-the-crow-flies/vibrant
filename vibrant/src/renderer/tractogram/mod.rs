pub mod adjacency;
pub mod density;
pub mod occlusion;
pub mod occupancy;
pub mod populate;
pub mod render;
pub mod shading;

use density::DensityPipeline;
use shading::volume::VolumeRenderPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::tractogram::Tractogram,
    controller::settings::{Settings, ShadingSetting},
    gpu::Gpu,
    renderer::tractogram::{
        adjacency::AdjacenyPipeline, occlusion::OcclusionPipeline, occupancy::OccupancyPipeline,
        populate::PopulatePipeline, render::TractogramRenderPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    adjaceny: AdjacenyPipeline,
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
            adjaceny: AdjacenyPipeline::new(gpu),
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
        tractogram: &Tractogram,
        settings: &Settings,
        needs_preprocess: bool,
    ) {
        if needs_preprocess {
            self.adjaceny.render(cmd, tractogram);
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
