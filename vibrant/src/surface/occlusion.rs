use wgpu::FilterMode;

use crate::{
    asset::scalar::{R32Float, ScalarTexture3D},
    gpu::Gpu,
};

pub struct Occlusion {
    ambient: ScalarTexture3D<R32Float>,
    directional: ScalarTexture3D<R32Float>,
}

impl Occlusion {
    pub fn new(gpu: &Gpu, resolution: u32) -> Self {
        Self {
            ambient: ScalarTexture3D::<R32Float>::new(gpu, resolution, FilterMode::Linear),
            directional: ScalarTexture3D::<R32Float>::new(gpu, resolution, FilterMode::Linear),
        }
    }

    pub fn resolution(&self) -> u32 {
        self.ambient().size()
    }

    pub fn ambient(&self) -> &ScalarTexture3D<R32Float> {
        &self.ambient
    }

    pub fn directional(&self) -> &ScalarTexture3D<R32Float> {
        &self.directional
    }
}
