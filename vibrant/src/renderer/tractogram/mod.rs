use density::TractogramDensityAddCompute;
use geometry::{
    line::{hardware::TractogramLineHardwareGeometry, software::TractogramLineSoftwareGeometry},
    tube::TractogramTubeGeometry,
};
use occlusion::TractogramOcclusionCompute;
use shading::{
    gbuffer::TractogramGBufferShading, simple::TractogramSimpleShading,
    tracing::TractogramTracingShading, volume::TractogramDensityShading,
};
use wgpu::CommandEncoder;

use crate::{
    asset::{occlusion::Occlusion, Density, Tractogram},
    controller::settings::{GeometrySetting, Settings, ShadingSetting},
    gpu::Gpu,
    surface::SurfaceBuffer,
};

use super::{constants::Constants, environment::Environment};

pub mod density;
pub mod geometry;
pub mod occlusion;
pub mod shading;

pub struct TractogramRenderer {
    line_hardware_geometry: TractogramLineHardwareGeometry,
    line_software_geometry: TractogramLineSoftwareGeometry,
    tube_geometry: TractogramTubeGeometry,
    tracing_shading: TractogramTracingShading,
    simple_shading: TractogramSimpleShading,
    volume_shading: TractogramDensityShading,
    gbuffer_shading: TractogramGBufferShading,
    density_compute: TractogramDensityAddCompute,
    occlusion_compute: TractogramOcclusionCompute,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        Self {
            line_hardware_geometry: TractogramLineHardwareGeometry::new(gpu),
            line_software_geometry: TractogramLineSoftwareGeometry::new(gpu, constants),
            tube_geometry: TractogramTubeGeometry::new(gpu),
            tracing_shading: TractogramTracingShading::new(gpu, constants),
            simple_shading: TractogramSimpleShading::new(gpu),
            volume_shading: TractogramDensityShading::new(gpu, constants),
            gbuffer_shading: TractogramGBufferShading::new(gpu),
            density_compute: TractogramDensityAddCompute::new(gpu, constants),
            occlusion_compute: TractogramOcclusionCompute::new(gpu, constants),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        buffer: &SurfaceBuffer,
        tractogram: &Tractogram,
        density: &Density,
        occlusion: &Occlusion,
        settings: &Settings,
    ) {
        if settings.shading != ShadingSetting::Simple {
            self.density_compute
                .render(cmd, environment, tractogram, density);

            self.occlusion_compute
                .render(cmd, environment, density, occlusion, tractogram);
        }

        if settings.shading != ShadingSetting::Density
            && settings.shading != ShadingSetting::Occlusion
        {
            let filter = if settings.culling && settings.shading != ShadingSetting::Simple {
                tractogram.filter_culling()
            } else {
                tractogram.filter_default()
            };

            match settings.geometry {
                GeometrySetting::LineHardware => {
                    self.line_hardware_geometry
                        .render(cmd, environment, buffer, tractogram, filter)
                }
                GeometrySetting::LineSoftware => {
                    self.line_software_geometry.render(
                        cmd,
                        buffer,
                        environment,
                        tractogram,
                        filter,
                    );
                }
                GeometrySetting::Tube => {
                    self.tube_geometry
                        .render(cmd, environment, buffer, tractogram, filter)
                }
            }
        }

        match settings.shading {
            ShadingSetting::Tracing => {
                self.tracing_shading
                    .render(cmd, environment, buffer, density, tractogram)
            }
            ShadingSetting::Simple => self.simple_shading.render(cmd, buffer, tractogram),
            ShadingSetting::Density => {
                self.volume_shading
                    .render(cmd, buffer, environment, density.texture())
            }
            ShadingSetting::Occlusion => {
                self.volume_shading
                    .render(cmd, buffer, environment, occlusion.ping())
            }
            ShadingSetting::GBuffer => self.gbuffer_shading.render(cmd, buffer, tractogram),
        }
    }
}
