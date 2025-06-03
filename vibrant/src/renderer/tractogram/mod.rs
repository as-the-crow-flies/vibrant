pub mod density;
pub mod occlusion;
pub mod occupancy;
pub mod render;
pub mod shading;

use density::DensityPipeline;
use shading::density::VolumeShadingPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::tractogram::Tractogram,
    controller::settings::{Settings, ShadingSetting},
    gpu::Gpu,
    renderer::tractogram::{occlusion::OcclusionPipeline, occupancy::OccupancyPipeline},
    surface::SurfaceBuffer,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    density: DensityPipeline,
    occlusion: OcclusionPipeline,
    occupancy: OccupancyPipeline,
    volume: VolumeShadingPipeline,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            density: DensityPipeline::new(gpu),
            occlusion: OcclusionPipeline::new(gpu),
            occupancy: OccupancyPipeline::new(gpu),
            volume: VolumeShadingPipeline::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &SurfaceBuffer,
        tractogram: &Tractogram,
        settings: &Settings,
    ) {
        self.density.render(cmd, frame, environment, tractogram);
        self.occlusion.render(cmd, frame, environment);
        self.occupancy.render(cmd, frame, environment);

        match settings.shading {
            ShadingSetting::Render => todo!(),
            ShadingSetting::Density => {
                self.volume
                    .render(cmd, frame, frame.density().volume(), environment)
            }
            ShadingSetting::Occlusion => {
                self.volume
                    .render(cmd, frame, frame.occlusion().volume(), environment)
            }
            ShadingSetting::Occupancy => {
                self.volume
                    .render(cmd, frame, frame.occupancy().occupancy(), environment)
            }
        }
    }
}
