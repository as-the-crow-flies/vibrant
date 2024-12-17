pub mod density;
pub mod geometry;
pub mod shading;

use density::TractogramDensityAddCompute;
use geometry::{line::hardware::TractogramLineHardwareGeometry, tube::TractogramTubeGeometry};
use shading::{
    gbuffer::TractogramGBufferShading, simple::TractogramSimpleShading,
    tracing::TractogramTracingShading,
};
use wgpu::CommandEncoder;

use crate::{
    asset::{density::Density, tractogram::Tractogram},
    controller::settings::{GeometrySetting, Settings, ShadingSetting},
    gpu::Gpu,
    surface::SurfaceBuffer,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    density_compute: TractogramDensityAddCompute,
    line_hardware_geometry: TractogramLineHardwareGeometry,
    tube_geometry: TractogramTubeGeometry,
    simple_shading: TractogramSimpleShading,
    gbuffer_shading: TractogramGBufferShading,
    tracing_shading: TractogramTracingShading,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            density_compute: TractogramDensityAddCompute::new(gpu),
            line_hardware_geometry: TractogramLineHardwareGeometry::new(gpu),
            tube_geometry: TractogramTubeGeometry::new(gpu),
            simple_shading: TractogramSimpleShading::new(gpu),
            gbuffer_shading: TractogramGBufferShading::new(gpu),
            tracing_shading: TractogramTracingShading::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        buffer: &SurfaceBuffer,
        density: &Density,
        tractogram: &Tractogram,
        settings: &Settings,
    ) {
        self.density_compute
            .render(cmd, environment, tractogram, density);

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
            ShadingSetting::Tracing => {
                self.tracing_shading
                    .render(cmd, environment, buffer, density, tractogram)
            }
        }
    }
}
