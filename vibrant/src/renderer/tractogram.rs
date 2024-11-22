use density::TractogramDensityComputeRenderer;
use line::render::TractogramLineGeometry;
use occlusion::TractogramOcclusionComputeRenderer;
use shading::{simple::TractogramSimpleShading, tracing::TractogramTracingShading};
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
    density: TractogramDensityComputeRenderer,
    occlusion: TractogramOcclusionComputeRenderer,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        Self {
            line_geometry: TractogramLineGeometry::new(gpu),
            tube_geometry: TractogramTubeGeometry::new(gpu),
            tracing_shading: TractogramTracingShading::new(gpu, constants),
            simple_shading: TractogramSimpleShading::new(gpu),
            density: TractogramDensityComputeRenderer::new(gpu, constants),
            occlusion: TractogramOcclusionComputeRenderer::new(gpu, constants),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        env: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
        occlusion: &Occlusion,
        settings: &Settings,
    ) {
        if settings.shading == Shading::Tracing {
            self.density.render(cmd, env, tractogram, density);

            if settings.culling == Culling::On {
                self.occlusion
                    .render(cmd, env, density, occlusion, tractogram);
            }
        }

        let filter = if settings.shading == Shading::Tracing && settings.culling == Culling::On {
            tractogram.filter_culling()
        } else {
            tractogram.filter_default()
        };

        match settings.geometry {
            Geometry::Line => self
                .line_geometry
                .render(cmd, env, frame, tractogram, filter),
            Geometry::Tube => self
                .tube_geometry
                .render(cmd, env, frame, tractogram, filter),
        }

        match settings.shading {
            Shading::Tracing => self
                .tracing_shading
                .render(cmd, env, frame, density, tractogram),
            Shading::Simple => self.simple_shading.render(cmd, frame, tractogram),
        }
    }
}
