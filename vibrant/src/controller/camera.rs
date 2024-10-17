use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec2, Vec3};

use super::state::ControllerState;

pub struct Camera {
    yaw: f32,
    pitch: f32,
    distance: f32,
    pan: Vec3,
    fov: f32,
    near: f32,
    far: f32,
    aspect: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            distance: 1.0,
            pan: Vec3::ZERO,
            fov: PI / 4.0,
            near: 0.01,
            far: 4.0,
            aspect: 1.0,
        }
    }

    pub fn update(&mut self, state: &ControllerState) {
        self.aspect(state.size);

        if state.shift {
            return;
        }

        if state.left {
            let rotation = state.relative_delta() * 10.0;
            self.rotate(-rotation.x, -rotation.y);
        }

        if state.right {
            let pan = state.relative_delta();
            self.pan(pan.x, -pan.y);
        }

        self.zoom(-state.scroll.y * 0.01);
    }

    pub fn projection(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect, self.near, self.far) * self.view()
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_x(self.pitch) * Quat::from_rotation_y(self.yaw)
    }

    pub fn view(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation(), Vec3::Z * self.distance)
            * Mat4::from_translation(self.pan)
    }

    pub fn transform(&self) -> Mat4 {
        self.view().inverse()
    }

    pub fn aspect(&mut self, size: Vec2) {
        self.aspect = size.x / size.y;
    }

    pub fn zoom(&mut self, zoom: f32) {
        self.distance = (self.distance + zoom).clamp(self.near, self.far * 0.5);
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch = (self.pitch + pitch).clamp(-PI / 2.0, PI / 2.0)
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
