pub mod geometry;
pub mod shading;

use geometry::{line::hardware::TractogramLineHardwareGeometry, tube::TractogramTubeGeometry};
use shading::{gbuffer::TractogramGBufferShading, simple::TractogramSimpleShading};
use wgpu::CommandEncoder;

use crate::{
    asset::tractogram::Tractogram,
    controller::settings::{GeometrySetting, Settings, ShadingSetting},
    gpu::Gpu,
    surface::SurfaceBuffer,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    line_hardware_geometry: TractogramLineHardwareGeometry,
    tube_geometry: TractogramTubeGeometry,
    simple_shading: TractogramSimpleShading,
    gbuffer_shading: TractogramGBufferShading,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            line_hardware_geometry: TractogramLineHardwareGeometry::new(gpu),
            tube_geometry: TractogramTubeGeometry::new(gpu),
            simple_shading: TractogramSimpleShading::new(gpu),
            gbuffer_shading: TractogramGBufferShading::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        buffer: &SurfaceBuffer,
        tractogram: &Tractogram,
        settings: &Settings,
    ) {
        match settings.geometry {
            GeometrySetting::LineHardware => self.line_hardware_geometry.render(
                cmd,
                environment,
                buffer,
                tractogram,
                tractogram.filter_default(),
            ),
            GeometrySetting::Tube => self.tube_geometry.render(
                cmd,
                environment,
                buffer,
                tractogram,
                tractogram.filter_default(),
            ),
        }

        match settings.shading {
            ShadingSetting::Simple => self.simple_shading.render(cmd, buffer, tractogram),
            ShadingSetting::GBuffer => self.gbuffer_shading.render(cmd, buffer, tractogram),
        }
    }
}
