use glam::Vec3;

use crate::controller::{camera::Camera, state::ControllerState};

#[derive(Debug)]
pub struct Segment {
    position: Vec3,
    radius: f32,
}

impl Segment {
    pub fn new() -> Self {
        Segment {
            position: Vec3::ZERO,
            radius: 0.0,
        }
    }

    pub fn update(&mut self, state: &ControllerState, camera: &Camera) -> bool {
        let uv = state.position / state.size() * 2.0 - 1.0;
        let uv_new = (state.position + state.delta) / state.size() * 2.0 - 1.0;

        let projection_inverse = camera.projection().inverse();

        let near = projection_inverse.project_point3(Vec3::new(uv.x, uv.y, 0.0));
        let far = projection_inverse.project_point3(Vec3::new(uv.x, uv.y, 1.0));

        let origin = near;
        let direction = (far - near).normalize();

        let origin_new = projection_inverse.project_point3(Vec3::new(uv_new.x, uv_new.y, 0.0));

        let hit = self.intersection_depth(origin, direction);

        if let Some(depth) = hit {
            let original_hit = origin + direction * depth;
            let new_hit = origin_new + direction * depth;

            self.position += new_hit - original_hit;

            self.radius = (self.radius + 0.1 * state.scroll.y).clamp(0.01, 0.5);
        }

        hit.is_some()
    }

    fn intersection_depth(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        let oc = origin - self.position;
        let b = oc.dot(direction);
        let c = oc.dot(oc) - self.radius * self.radius;
        let h = b * b - c;

        if h < 0.0 {
            return None;
        }

        return Some(-b - h.sqrt());
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }
}
