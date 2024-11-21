use density::TractogramDensityComputeRenderer;
use line::render::TractogramLineRenderRenderer;
use occlusion::TractogramOcclusionComputeRenderer;
use shading::TractogramFullRenderer;
use tube::{impostor::TractogramTubeImpostorRenderer, raycast::TractogramTubeRaycastRenderer};
use wgpu::CommandEncoder;

use crate::{
    asset::{occlusion::Occlusion, Density, Tractogram},
    controller::settings::Geometry,
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
    line_render: TractogramLineRenderRenderer,
    tube_impostor: TractogramTubeImpostorRenderer,
    tube_raycast: TractogramTubeRaycastRenderer,
    full: TractogramFullRenderer,
    density: TractogramDensityComputeRenderer,
    occlusion: TractogramOcclusionComputeRenderer,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        Self {
            line_render: TractogramLineRenderRenderer::new(gpu),
            tube_impostor: TractogramTubeImpostorRenderer::new(gpu),
            tube_raycast: TractogramTubeRaycastRenderer::new(gpu),
            full: TractogramFullRenderer::new(gpu, constants),
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
        renderer: &Geometry,
    ) {
        self.density.render(cmd, env, tractogram, density);
        self.occlusion
            .render(cmd, env, density, occlusion, tractogram);

        match renderer {
            Geometry::LineRender => self.line_render.render(cmd, env, frame, tractogram),
            Geometry::TubeImpostor => self.tube_impostor.render(cmd, env, frame, tractogram),
            Geometry::TubeRaycast => self.tube_raycast.render(cmd, env, frame, tractogram),
        }

        self.full.render(cmd, env, frame, density, tractogram)
    }
}
