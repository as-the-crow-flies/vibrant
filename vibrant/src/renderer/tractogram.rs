use density::compute::TractogramDensityComputeRenderer;
use line::{compute::TractogramLineComputeRenderer, render::TractogramLineRenderRenderer};
use shading::voxel_cone_tracing::TractogramFullRenderer;
use tube::{impostor::TractogramTubeImpostorRenderer, raycast::TractogramTubeRaycastRenderer};
use wgpu::CommandEncoder;

use crate::{
    asset::{Density, Tractogram},
    controller::settings::{Renderer, Shader},
    gpu::Gpu,
    surface::Frame,
};

use super::{constants::Constants, environment::Environment};

pub mod density;
pub mod line;
pub mod shading;
pub mod tube;

pub struct TractogramRenderer {
    line_render: TractogramLineRenderRenderer,
    line_compute: TractogramLineComputeRenderer,
    tube_impostor: TractogramTubeImpostorRenderer,
    tube_raycast: TractogramTubeRaycastRenderer,
    full: TractogramFullRenderer,
    density: TractogramDensityComputeRenderer,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        Self {
            line_render: TractogramLineRenderRenderer::new(gpu),
            line_compute: TractogramLineComputeRenderer::new(gpu, constants),
            tube_impostor: TractogramTubeImpostorRenderer::new(gpu),
            tube_raycast: TractogramTubeRaycastRenderer::new(gpu),
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
        shader: &Shader,
    ) {
        self.density.render(cmd, env, tractogram, density);

        match renderer {
            Renderer::LineRender => self.line_render.render(cmd, env, frame, tractogram),
            Renderer::TubeImpostor => self.tube_impostor.render(cmd, env, frame, tractogram),
            Renderer::TubeRaycast => self.tube_raycast.render(cmd, env, frame, tractogram),
        }

        match shader {
            Shader::AmbientOcclusion => self.full.render(cmd, env, frame, density, tractogram),
        }
    }
}
