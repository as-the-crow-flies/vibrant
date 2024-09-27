use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec3};
use winit::dpi::PhysicalSize;

pub struct Camera {
    yaw: f32,
    pitch: f32,
    zoom: f32,
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
            zoom: 200.0,
            pan: Vec3::ZERO,
            fov: PI / 4.0,
            near: 0.1,
            far: 10000.0,
            aspect: 1.0,
        }
    }

    pub fn projection(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect, self.near, self.far)
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_x(self.pitch) * Quat::from_rotation_y(self.yaw)
    }

    pub fn mvp(&self) -> Mat4 {
        self.projection()
            * Mat4::from_rotation_translation(self.rotation(), Vec3::Z * self.zoom)
            * Mat4::from_translation(self.pan)
    }

    pub fn aspect(&mut self, size: PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;
    }

    pub fn zoom(&mut self, zoom: f32) {
        self.zoom = (self.zoom + self.zoom.sqrt() * zoom).clamp(self.near * 10.0, self.far * 0.1);
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch = (self.pitch + pitch).clamp(-PI / 2.0, PI / 2.0)
    }

    pub fn pan(&mut self, x: f32, y: f32) {
        self.pan += self.rotation().inverse().mul_vec3(Vec3::new(x, y, 0.0));
    }

    pub fn update(&self, dt: f32) {}
}
