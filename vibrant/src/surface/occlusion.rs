use wgpu::FilterMode;

use crate::{
    asset::scalar::{R32Float, ScalarTexture3D},
    gpu::Gpu,
};

pub struct Occlusion {
    occlusion: ScalarTexture3D<R32Float>,
}

impl Occlusion {
    pub fn new(gpu: &Gpu, volume: u32) -> Self {
        Self {
            occlusion: ScalarTexture3D::<R32Float>::new(gpu, volume, FilterMode::Linear),
        }
    }

    pub fn resolution(&self) -> u32 {
        self.texture().size()
    }

    pub fn texture(&self) -> &ScalarTexture3D<R32Float> {
        &self.occlusion
    }
}
