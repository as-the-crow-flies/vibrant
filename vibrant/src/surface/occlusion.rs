use glam::Mat4;
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
            volume: ScalarTexture3D::<R8Unorm>::new(
                gpu,
                volume,
                volume,
                volume,
                Mat4::IDENTITY,
                FilterMode::Linear,
            ),
        }
    }

    pub fn volume(&self) -> &ScalarTexture3D<R8Unorm> {
        &self.volume
    }
}
