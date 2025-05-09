use culling::TractogramCullingShading;
use density::TractogramDensityShading;
use gbuffer::TractogramGBufferShading;
use simple::TractogramSimpleShading;

use tracing::TractogramTracingShading;
use wgpu::CommandEncoder;

use crate::{
    asset::tractogram::Tractogram,
    controller::settings::{Settings, ShadingSetting},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::SurfaceBuffer,
};

pub mod culling;
pub mod density;
pub mod gbuffer;
pub mod simple;
pub mod tracing;

pub struct TractogramShadingRenderer {
    simple: TractogramSimpleShading,
    gbuffer: TractogramGBufferShading,
    density: TractogramDensityShading,
    culling: TractogramCullingShading,
    tracing: TractogramTracingShading,
}

impl TractogramShadingRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            simple: TractogramSimpleShading::new(gpu),
            gbuffer: TractogramGBufferShading::new(gpu),
            density: TractogramDensityShading::new(gpu),
            culling: TractogramCullingShading::new(gpu),
            tracing: TractogramTracingShading::new(gpu),
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
        match settings.shading {
            ShadingSetting::Simple => self.simple.render(cmd, buffer, tractogram),
            ShadingSetting::GBuffer => self.gbuffer.render(cmd, buffer, environment, tractogram),
            ShadingSetting::Density => self.density.render(cmd, buffer, environment),
            ShadingSetting::Tracing => self.tracing.render(cmd, environment, buffer, tractogram),
            ShadingSetting::Culling => self.culling.render(cmd, environment, buffer),
        }
    }
}
