use density::compute::TractogramDensityComputeRenderer;
use full::render::TractogramFullRenderer;
use line::{compute::TractogramLineComputeRenderer, render::TractogramLineRenderRenderer};
use tube::render::TractogramTubeRenderRenderer;
use wgpu::CommandEncoder;

use crate::{
    asset::{Density, Tractogram},
    controller::settings::Renderer,
    gpu::Gpu,
    surface::Frame,
};

use super::{constants::Constants, environment::Environment};

pub mod density;
pub mod full;
pub mod line;
pub mod tube;

pub struct TractogramRenderer {
    line_render: TractogramLineRenderRenderer,
    line_compute: TractogramLineComputeRenderer,
    tube_render: TractogramTubeRenderRenderer,
    full: TractogramFullRenderer,
    density: TractogramDensityComputeRenderer,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        Self {
            line_render: TractogramLineRenderRenderer::new(gpu),
            line_compute: TractogramLineComputeRenderer::new(gpu, constants),
            tube_render: TractogramTubeRenderRenderer::new(gpu),
            full: TractogramFullRenderer::new(gpu, constants),
            density: TractogramDensityComputeRenderer::new(gpu, constants),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        env: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
        renderer: &Renderer,
    ) {
        self.density.render(cmd, env, tractogram, density);

        match renderer {
            Renderer::LineRender => self.line_render.render(cmd, env, frame, tractogram),
            Renderer::LineCompute => self.line_compute.render(cmd, env, frame, tractogram),
            Renderer::TubeRender => self.tube_render.render(cmd, env, frame, tractogram),
            Renderer::Full => self.full.render(cmd, env, frame, tractogram, density),
        }
    }
}
