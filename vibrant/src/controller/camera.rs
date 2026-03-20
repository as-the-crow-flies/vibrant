use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec2, Vec3};

use super::state::ControllerState;

// Halton low-discrepancy sequence for sub-pixel jitter
fn halton(index: u32, base: u32) -> f32 {
    let mut f = 1.0f32;
    let mut r = 0.0f32;
    let mut i = index;
    while i > 0 {
        f /= base as f32;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

#[derive(Debug)]
pub struct Camera {
    width: u32,
    height: u32,
    pub yaw: f32,
    pub pitch: f32,
    distance: f32,
    pan: Vec3,
    pub fov: f32,
    near: f32,
    far: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            width: 1,
            height: 1,
            yaw: 0.0,
            pitch: 0.0,
            distance: 0.75,
            pan: Vec3::ZERO,
            fov: PI / 3.0,
            near: 0.01,
            far: 10.0,
        }
    }

    pub fn update(&mut self, state: &ControllerState) {
        self.width = state.width;
        self.height = state.height;

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

        self.zoom(-0.1 * state.scroll.y);
    }

    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn projection(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect(), self.near, self.far) * self.view()
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

    pub fn zoom(&mut self, zoom: f32) {
        self.distance = (self.distance + zoom).clamp(0.01, 5.0);
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

    /// compute sub-pixel jitter for TAA
    pub fn jitter(frame_index: u32) -> Vec2 {
        let idx = frame_index % 16 + 1;
        Vec2::new(halton(idx, 2) - 0.5, halton(idx, 3) - 0.5)
    }

    pub fn projection_jittered(&self, jitter: Vec2) -> Mat4 {
        let mut proj = Mat4::perspective_lh(self.fov, self.aspect(), self.near, self.far);
        // apply sub-pixel jitter as a constant NDC offset (depth-independent).
        proj.z_axis.x += jitter.x * 2.0 / self.width as f32;
        proj.z_axis.y += jitter.y * 2.0 / self.height as f32;
        proj * self.view()
    }
}
