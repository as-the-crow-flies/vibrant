use std::f32::consts::PI;

use glam::{Quat, Vec3};

use super::state::ControllerState;

#[derive(Debug)]
pub struct Light {
    yaw: f32,
    pitch: f32,
    changed: bool,
}

impl Light {
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.5 * PI,
            changed: false,
        }
    }

    pub fn update(&mut self, state: &ControllerState) {
        self.changed = false;

        if state.left && state.shift {
            let rotation = state.relative_delta() * 10.0;
            self.rotate(-rotation.x, -rotation.y);

            self.changed = true;
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
        self.pitch = (self.pitch + pitch).clamp(0.0, PI)
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
