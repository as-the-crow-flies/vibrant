use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec3};

use super::state::ControllerState;

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

    // Animation state
    animating: bool,
    anim_start_yaw: f32,
    anim_start_pitch: f32,
    anim_target_yaw: f32,
    anim_target_pitch: f32,
    anim_elapsed: f32,
    anim_duration: f32,
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

            animating: false,
            anim_start_yaw: 0.0,
            anim_start_pitch: 0.0,
            anim_target_yaw: 0.0,
            anim_target_pitch: 0.0,
            anim_elapsed: 0.0,
            anim_duration: 0.0,
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

        // Cancel animation on any user input
        if state.left
            || state.right
            || state.middle
            || state.scroll.y.abs() > 0.0
            || state.backspace
        {
            self.cancel_animation();
        }
    }

    /// Start a smooth animation to the given yaw/pitch over `duration` seconds.
    pub fn animate_to(&mut self, target_yaw: f32, target_pitch: f32, duration: f32) {
        self.animating = true;
        self.anim_start_yaw = self.yaw;
        self.anim_start_pitch = self.pitch;
        self.anim_target_yaw = target_yaw;
        self.anim_target_pitch = target_pitch;
        self.anim_elapsed = 0.0;
        self.anim_duration = duration;
    }

    /// Advance the animation by `dt` seconds. Returns true if still animating.
    pub fn tick_animation(&mut self, dt: f32) -> bool {
        if !self.animating {
            return false;
        }

        self.anim_elapsed += dt;
        let t = (self.anim_elapsed / self.anim_duration).min(1.0);

        // Smoothstep ease-in-out: 3t^2 - 2t^3
        let s = t * t * (3.0 - 2.0 * t);

        self.yaw = self.anim_start_yaw + (self.anim_target_yaw - self.anim_start_yaw) * s;
        self.pitch = self.anim_start_pitch + (self.anim_target_pitch - self.anim_start_pitch) * s;

        if t >= 1.0 {
            self.yaw = self.anim_target_yaw;
            self.pitch = self.anim_target_pitch;
            self.animating = false;
        }

        self.animating
    }

    /// Cancel any in-progress animation.
    pub fn cancel_animation(&mut self) {
        self.animating = false;
    }

    /// Returns true if the camera is currently animating.
    pub fn is_animating(&self) -> bool {
        self.animating
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
}
