use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec3};

use super::state::ControllerState;

#[derive(Debug)]
pub struct Camera {
    pub aspect: f32,
    pub fov: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub pan: Vec3,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            aspect: 1.0,
            yaw: 0.0,
            pitch: -0.5 * PI,
            distance: 300.0,
            pan: Vec3::ZERO,
            fov: 0.8,
            near: 1.0,
            far: 10000.0,
        }
    }

    pub fn update(&mut self, state: &ControllerState) {
        self.aspect = state.width as f32 / state.height as f32;

        if state.shift {
            return;
        }

        if state.backspace {
            self.yaw = 0.0;
            self.pitch = 0.0;
            self.distance = 0.75;
            self.pan = Vec3::ZERO;
        }

        if state.left {
            let rotation = state.relative_delta() * 10.0;
            self.rotate(-rotation.x, -rotation.y);
        }

        if state.right {
            let pan = state.relative_delta();
            self.pan(pan.x, -pan.y);
        }

        if state.middle {
            self.zoom(state.relative_delta().y);
        }

        self.zoom(-10.0 * state.scroll.y);
    }

    pub fn projection(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect, self.near, self.far) * self.view()
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_x(self.pitch) * Quat::from_rotation_z(self.yaw)
    }

    pub fn view(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation(), Vec3::Z * self.distance)
            * Mat4::from_translation(self.pan)
    }

    pub fn transform(&self) -> Mat4 {
        self.view().inverse()
    }

    pub fn zoom(&mut self, zoom: f32) {
        self.distance = (self.distance + zoom).clamp(1.0, 10000.0);
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch += pitch;
    }

    pub fn pan(&mut self, x: f32, y: f32) {
        self.pan += self.rotation().inverse().mul_vec3(Vec3::new(x, y, 0.0)) * self.distance;
    }

    pub fn near(&self) -> f32 {
        self.near
    }

    pub fn far(&self) -> f32 {
        self.far
    }
}
