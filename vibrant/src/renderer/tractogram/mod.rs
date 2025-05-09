pub mod density;
pub mod geometry;
pub mod shading;
pub mod slice;

use density::TractogramDensityPipeline;
use geometry::{
    line::TractogramLineGeometry, transparency::TractogramTransparentGeometry,
    tube::TractogramTubeGeometry,
};
use shading::TractogramShadingRenderer;
use slice::TractogramSlicePipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::tractogram::Tractogram,
    controller::settings::{GeometrySetting, Settings},
    gpu::Gpu,
    surface::SurfaceBuffer,
};

use super::environment::Environment;

pub struct TractogramRenderer {
    density: TractogramDensityPipeline,
    slice: TractogramSlicePipeline,
    line_geometry: TractogramLineGeometry,
    tube_geometry: TractogramTubeGeometry,
    transparency_geometry: TractogramTransparentGeometry,
    shading: TractogramShadingRenderer,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            density: TractogramDensityPipeline::new(gpu),
            slice: TractogramSlicePipeline::new(gpu),
            line_geometry: TractogramLineGeometry::new(gpu),
            tube_geometry: TractogramTubeGeometry::new(gpu),
            transparency_geometry: TractogramTransparentGeometry::new(gpu),
            shading: TractogramShadingRenderer::new(gpu),
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

        self.slice.render(cmd, buffer, environment, tractogram);

        match settings.geometry {
            GeometrySetting::Line => self.line_geometry.render(
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
            GeometrySetting::Transparency => self.transparency_geometry.render(
                cmd,
                environment,
                buffer,
                tractogram,
                tractogram.filter_culling(),
            ),
        }

        if settings.geometry != GeometrySetting::Transparency {
            self.shading
                .render(cmd, environment, buffer, tractogram, settings);
        }
    }
}
