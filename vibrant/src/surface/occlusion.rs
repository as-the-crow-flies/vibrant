use wgpu::FilterMode;

use crate::{
    asset::scalar::{R8Unorm, ScalarTexture3D},
    gpu::Gpu,
};

pub struct Occlusion {
    volume: ScalarTexture3D<R8Unorm>,
}

impl Occlusion {
    pub fn new(gpu: &Gpu, volume: u32) -> Self {
        Self {
            volume: ScalarTexture3D::<R8Unorm>::new(gpu, volume, FilterMode::Linear),
        }
    }

    pub fn resolution(&self) -> u32 {
        self.volume().width()
    }

    pub fn volume(&self) -> &ScalarTexture3D<R8Unorm> {
        &self.volume
    }
}
