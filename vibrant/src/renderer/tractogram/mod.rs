pub mod density;
pub mod occlusion;
pub mod render;
pub mod shading;

use density::TractogramDensityPipeline;
use shading::density::TractogramVolumeShadingPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::tractogram::Tractogram, controller::settings::Settings, gpu::Gpu, surface::SurfaceBuffer,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    density: TractogramDensityPipeline,
    volume: TractogramVolumeShadingPipeline,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            density: TractogramDensityPipeline::new(gpu),
            volume: TractogramVolumeShadingPipeline::new(gpu),
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
        self.volume
            .render(cmd, frame, frame.density().density(), environment);
    }
}
