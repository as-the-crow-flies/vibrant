use wgpu::CommandEncoder;

use crate::{
    asset::line::LineSet,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::rasterization::{
            opaque::LineOpaqueRasterizationPipeline, sort::LineRasterizationSortPipeline,
            transparent::LineTransparentRasterizationPipeline,
        },
    },
    surface::Frame,
};

pub mod opaque;
pub mod sort;
pub mod transparent;

pub struct LineRasterizationPipeline {
    sort: LineRasterizationSortPipeline,
    transparent: LineTransparentRasterizationPipeline,
    opaque: LineOpaqueRasterizationPipeline,
}

impl LineRasterizationPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            sort: LineRasterizationSortPipeline::new(gpu),
            transparent: LineTransparentRasterizationPipeline::new(gpu),
            opaque: LineOpaqueRasterizationPipeline::new(gpu),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
        settings: &Settings,
    ) {
        self.sort.dispatch(cmd, environment, line);

        if settings.alpha == 1.0 {
            self.opaque.render(cmd, frame, environment, line, settings);
        } else {
            self.transparent
                .render(cmd, frame, environment, line, settings);
        }
    }
}
