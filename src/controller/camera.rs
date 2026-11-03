use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec3, Vec4};

use super::state::ControllerState;

#[derive(Debug)]
pub struct Camera {
    pub aspect: f32,
    fov: f32,
    // Last non-zero fov; relates `distance` to the framing at the pivot.
    framing: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub pan: Vec3,
    pub near: f32,
    pub far: f32,
    changed: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            aspect: 1.0,
            yaw: 0.0,
            pitch: -0.5 * PI,
            distance: 300.0,
            pan: Vec3::ZERO,
            fov: 0.5,
            framing: 0.5,
            near: 1.0,
            far: 10000.0,
            changed: false,
        }
    }

    /// Returns whether the view moved since the last call, clearing the flag.
    pub fn take_changed(&mut self) -> bool {
        std::mem::replace(&mut self.changed, false)
    }

    /// Set the projection aspect ratio, flagging the view as changed when it
    /// actually moves so accumulation restarts (e.g. the side panel was resized).
    pub fn set_aspect(&mut self, aspect: f32) {
        if aspect.is_finite() && (self.aspect - aspect).abs() > 1e-4 {
            self.changed = true;
            self.aspect = aspect;
        }
    }

    pub fn update(&mut self, state: &ControllerState) {
        // `aspect` is set from the viewport rect in `Controller::ui` each frame.

        if state.shift {
            return;
        }

        if state.backspace {
            self.yaw = 0.0;
            self.pitch = 0.0;
            self.distance = 0.75;
            self.pan = Vec3::ZERO;
            self.changed = true;
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

    pub fn fov(&self) -> f32 {
        self.fov
    }

    pub fn orthographic(&self) -> bool {
        self.fov <= 0.0
    }

    /// Change the field of view, dollying so the pivot plane keeps its framing.
    /// A fov of zero switches to an orthographic projection.
    pub fn set_fov(&mut self, fov: f32) {
        if fov != self.fov {
            self.changed = true;
            if fov > 0.0 {
                self.distance *= (0.5 * self.framing).tan() / (0.5 * fov).tan();
                self.framing = fov;
            }
            self.fov = fov;
        }
    }

    /// View space -> clip space. The orthographic view matches the perspective
    /// framing at the pivot; its near plane sits behind the camera so zooming
    /// in never clips. `far` grows with `distance`, which gets large as the fov
    /// approaches zero.
    pub fn projection(&self) -> Mat4 {
        let far = self.far + self.distance;
        if self.orthographic() {
            let h = self.distance * (0.5 * self.framing).tan();
            let w = h * self.aspect;
            Mat4::orthographic_lh(-w, w, -h, h, -far, far)
        } else {
            Mat4::perspective_lh(self.fov, self.aspect, self.near, far)
        }
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_x(self.pitch) * Quat::from_rotation_z(self.yaw)
    }

    pub fn view(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation(), Vec3::Z * self.distance)
            * Mat4::from_translation(self.pan)
    }

    /// View space -> world space.
    pub fn transform(&self) -> Mat4 {
        (self.view() * Mat4::from_diagonal(Vec4::new(-1.0, 1.0, 1.0, 1.0))).inverse() // Flip Left/Right
    }

    pub fn zoom(&mut self, zoom: f32) {
        if zoom != 0.0 {
            self.changed = true;
        }
        self.distance = (self.distance + zoom).max(1.0);
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        if yaw != 0.0 || pitch != 0.0 {
            self.changed = true;
        }
        self.yaw += yaw;
        self.pitch += pitch;
    }

    pub fn pan(&mut self, x: f32, y: f32) {
        if x != 0.0 || y != 0.0 {
            self.changed = true;
        }
        self.pan += self.rotation().inverse().mul_vec3(Vec3::new(x, y, 0.0)) * self.distance;
    }

    pub fn near(&self) -> f32 {
        self.near
    }

    pub fn far(&self) -> f32 {
        self.far
    }
}
