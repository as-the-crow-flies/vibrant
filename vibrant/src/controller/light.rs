use std::f32::consts::PI;

use glam::{Quat, Vec3};

use super::state::ControllerState;

#[derive(Debug, Default)]
pub struct Light {
    yaw: f32,
    pitch: f32,
}

impl Light {
    pub fn update(&mut self, state: &ControllerState) {
        if state.left && state.shift {
            let rotation = state.relative_delta() * 10.0;
            self.rotate(-rotation.x, -rotation.y);
        }
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_x(self.pitch) * Quat::from_rotation_z(self.yaw)
    }

    pub fn direction(&self) -> Vec3 {
        self.rotation().mul_vec3(Vec3::Y).normalize()
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch = (self.pitch + pitch).clamp(-PI / 2.0, PI / 2.0)
    }
}
