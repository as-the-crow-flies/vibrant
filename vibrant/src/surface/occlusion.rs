use glam::{Mat4, Quat, Vec3};

use crate::{
    asset::scalar::{ScalarTexture2D, ScalarTexture3D},
    gpu::Gpu,
};

pub struct Occlusion {
    volume: ScalarTexture3D,
    texture: ScalarTexture2D,
}

impl Occlusion {
    pub fn new(gpu: &Gpu, width: u32, height: u32, depth: u32) -> Self {
        let projection_to_occlusion = Mat4::from_scale_rotation_translation(
            Vec3::new(0.5 * width as f32, 0.5 * height as f32, depth as f32),
            Quat::IDENTITY,
            Vec3::new(0.5 * width as f32 - 0.5, 0.5 * height as f32 - 0.5, -0.5),
        );

        Self {
            volume: ScalarTexture3D::new(gpu, width, height, depth, projection_to_occlusion),
            texture: ScalarTexture2D::new(gpu, width, height, 1, projection_to_occlusion),
        }
    }

    pub fn volume(&self) -> &ScalarTexture3D {
        &self.volume
    }

    pub fn texture(&self) -> &ScalarTexture2D {
        &self.texture
    }
}
