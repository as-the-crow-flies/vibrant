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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_animate_to_sets_state() {
        let mut cam = Camera::new();
        assert!(!cam.is_animating());

        cam.animate_to(1.0, 0.5, 0.4);
        assert!(cam.is_animating());
        assert_eq!(cam.anim_target_yaw, 1.0);
        assert_eq!(cam.anim_target_pitch, 0.5);
    }

    #[test]
    fn test_tick_animation_at_start() {
        let mut cam = Camera::new();
        cam.yaw = 0.0;
        cam.pitch = 0.0;
        cam.animate_to(1.0, 0.5, 1.0);

        // Tick a tiny amount — should still be near start
        cam.tick_animation(0.001);
        assert!(cam.yaw.abs() < 0.01);
        assert!(cam.pitch.abs() < 0.01);
        assert!(cam.is_animating());
    }

    #[test]
    fn test_tick_animation_at_end() {
        let mut cam = Camera::new();
        cam.yaw = 0.0;
        cam.pitch = 0.0;
        cam.animate_to(1.0, 0.5, 0.4);

        // Tick past the full duration
        let still_animating = cam.tick_animation(1.0);
        assert!(!still_animating);
        assert!(!cam.is_animating());
        assert!((cam.yaw - 1.0).abs() < 1e-6);
        assert!((cam.pitch - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_tick_animation_midpoint() {
        let mut cam = Camera::new();
        cam.yaw = 0.0;
        cam.pitch = 0.0;
        cam.animate_to(2.0, 1.0, 1.0);

        // At t=0.5, smoothstep = 3*(0.5)^2 - 2*(0.5)^3 = 0.5
        cam.tick_animation(0.5);
        assert!((cam.yaw - 1.0).abs() < 1e-6, "yaw at midpoint: {}", cam.yaw);
        assert!(
            (cam.pitch - 0.5).abs() < 1e-6,
            "pitch at midpoint: {}",
            cam.pitch
        );
    }

    #[test]
    fn test_cancel_animation() {
        let mut cam = Camera::new();
        cam.animate_to(1.0, 0.5, 1.0);
        cam.tick_animation(0.3);
        let yaw_before = cam.yaw;
        let pitch_before = cam.pitch;

        cam.cancel_animation();
        assert!(!cam.is_animating());

        // yaw/pitch should be preserved at the point of cancellation
        assert_eq!(cam.yaw, yaw_before);
        assert_eq!(cam.pitch, pitch_before);
    }

    #[test]
    fn test_rotate_pitch_clamp() {
        let mut cam = Camera::new();

        // Rotate pitch far positive — should clamp to PI/2
        cam.rotate(0.0, 100.0);
        assert!((cam.pitch - PI / 2.0).abs() < 1e-6);

        // Reset and rotate pitch far negative — should clamp to -PI/2
        cam.pitch = 0.0;
        cam.rotate(0.0, -100.0);
        assert!((cam.pitch + PI / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_zoom_clamp() {
        let mut cam = Camera::new();

        // Zoom way in — should clamp to 0.01
        cam.zoom(-100.0);
        assert!((cam.distance - 0.01).abs() < 1e-6);

        // Zoom way out — should clamp to 5.0
        cam.zoom(200.0);
        assert!((cam.distance - 5.0).abs() < 1e-6);
    }
}
