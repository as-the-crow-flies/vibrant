use density::TractogramDensityCompute;
use line::render::TractogramLineGeometry;
use occlusion::TractogramOcclusionCompute;
use shading::{
    density::TractogramDensityShading, simple::TractogramSimpleShading,
    tracing::TractogramTracingShading,
};
use tube::raycast::TractogramTubeGeometry;
use wgpu::CommandEncoder;

use crate::{
    asset::{occlusion::Occlusion, Density, Tractogram},
    controller::settings::{Culling, Geometry, Settings, Shading},
    gpu::Gpu,
    surface::Frame,
};

use super::{constants::Constants, environment::Environment};

pub mod density;
pub mod line;
pub mod occlusion;
pub mod shading;
pub mod tube;

pub struct TractogramRenderer {
    line_geometry: TractogramLineGeometry,
    tube_geometry: TractogramTubeGeometry,
    tracing_shading: TractogramTracingShading,
    simple_shading: TractogramSimpleShading,
    density_shading: TractogramDensityShading,
    density_compute: TractogramDensityCompute,
    occlusion_compute: TractogramOcclusionCompute,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        Self {
            line_geometry: TractogramLineGeometry::new(gpu),
            tube_geometry: TractogramTubeGeometry::new(gpu),
            tracing_shading: TractogramTracingShading::new(gpu, constants),
            simple_shading: TractogramSimpleShading::new(gpu),
            density_shading: TractogramDensityShading::new(gpu, constants),
            density_compute: TractogramDensityCompute::new(gpu, constants),
            occlusion_compute: TractogramOcclusionCompute::new(gpu, constants),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
        occlusion: &Occlusion,
        settings: &Settings,
    ) {
        if settings.shading != Shading::Simple {
            self.density_compute
                .render(cmd, environment, tractogram, density);

            if settings.shading != Shading::Density && settings.culling == Culling::On {
                self.occlusion_compute
                    .render(cmd, environment, density, occlusion, tractogram);
            }
        }

        if settings.shading != Shading::Density {
            let filter = if settings.shading == Shading::Tracing && settings.culling == Culling::On
            {
                tractogram.filter_culling()
            } else {
                tractogram.filter_default()
            };

            match settings.geometry {
                Geometry::Line => {
                    self.line_geometry
                        .render(cmd, environment, frame, tractogram, filter)
                }
                Geometry::Tube => {
                    self.tube_geometry
                        .render(cmd, environment, frame, tractogram, filter)
                }
            }
        }

        match settings.shading {
            Shading::Tracing => {
                self.tracing_shading
                    .render(cmd, environment, frame, density, tractogram)
            }
            Shading::Simple => self.simple_shading.render(cmd, frame, tractogram),
            Shading::Density => self
                .density_shading
                .render(cmd, frame, environment, density),
        }
    }
}
