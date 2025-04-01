pub mod density;
pub mod geometry;
pub mod occlusion;
pub mod shading;

use density::TractogramDensityPipeline;
use geometry::{line::TractogramLineGeometry, tube::TractogramTubeGeometry};
use occlusion::TractogramOcclusionPipeline;
use shading::{
    culling::TractogramCullingShading, density::TractogramDensityShading,
    gbuffer::TractogramGBufferShading, simple::TractogramSimpleShading,
    tracing::TractogramTracingShading,
};
use wgpu::CommandEncoder;

use crate::{
    asset::tractogram::Tractogram,
    controller::settings::{GeometrySetting, Settings, ShadingSetting},
    gpu::Gpu,
    surface::SurfaceBuffer,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    density: TractogramDensityPipeline,
    occlusion: TractogramOcclusionPipeline,
    line_hardware_geometry: TractogramLineGeometry,
    tube_geometry: TractogramTubeGeometry,
    simple_shading: TractogramSimpleShading,
    gbuffer_shading: TractogramGBufferShading,
    density_shading: TractogramDensityShading,
    culling_shading: TractogramCullingShading,
    tracing_shading: TractogramTracingShading,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            density: TractogramDensityPipeline::new(gpu),
            occlusion: TractogramOcclusionPipeline::new(gpu),
            line_hardware_geometry: TractogramLineGeometry::new(gpu),
            tube_geometry: TractogramTubeGeometry::new(gpu),
            simple_shading: TractogramSimpleShading::new(gpu),
            gbuffer_shading: TractogramGBufferShading::new(gpu),
            density_shading: TractogramDensityShading::new(gpu),
            culling_shading: TractogramCullingShading::new(gpu),
            tracing_shading: TractogramTracingShading::new(gpu),
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
        self.density
            .render(cmd, environment, tractogram, buffer.density());

        self.occlusion.render(cmd, buffer, environment, tractogram);

        if [
            ShadingSetting::Simple,
            ShadingSetting::Tracing,
            ShadingSetting::GBuffer,
        ]
        .contains(&settings.shading)
        {
            match settings.geometry {
                GeometrySetting::Line => self.line_hardware_geometry.render(
                    cmd,
                    environment,
                    buffer,
                    tractogram,
                    tractogram.filter_culling(),
                ),
                GeometrySetting::Tube => self.tube_geometry.render(
                    cmd,
                    environment,
                    buffer,
                    tractogram,
                    tractogram.filter_culling(),
                ),
            }
        }

        match settings.shading {
            ShadingSetting::Simple => self.simple_shading.render(cmd, buffer, tractogram),
            ShadingSetting::GBuffer => {
                self.gbuffer_shading
                    .render(cmd, buffer, environment, tractogram)
            }
            ShadingSetting::Density => self.density_shading.render(cmd, buffer, environment),
            ShadingSetting::Tracing => {
                self.tracing_shading
                    .render(cmd, environment, buffer, tractogram)
            }
            ShadingSetting::Culling => {
                self.culling_shading.render(cmd, buffer);
            }
        }
    }
}
