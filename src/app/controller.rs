pub mod camera;

use camera::Camera;
use glam::Vec2;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

#[derive(Debug, Default, Clone, Copy)]
pub struct ControllerState {
    pressed: bool,
    left: bool,
    right: bool,
    position: Vec2,
    delta: Vec2,
    scroll: Vec2,
}

impl ControllerState {
    fn update(&self, event: WindowEvent) -> Self {
        let default = ControllerState {
            delta: Vec2::default(),
            scroll: Vec2::default(),
            ..self.clone()
        };

        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => ControllerState {
                position: Vec2::new(position.x as f32, position.y as f32),
                delta: Vec2::new(position.x as f32, position.y as f32) - self.position,
                ..default
            },
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Pressed,
                button: MouseButton::Left,
            } => ControllerState {
                pressed: true,
                left: true,
                ..default
            },
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Released,
                button: MouseButton::Left,
            } => ControllerState {
                pressed: false,
                left: false,
                ..default
            },
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Pressed,
                button: MouseButton::Right,
            } => ControllerState {
                pressed: true,
                right: true,
                ..default
            },
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Released,
                button: MouseButton::Right,
            } => ControllerState {
                pressed: false,
                right: false,
                ..default
            },
            WindowEvent::MouseWheel {
                device_id: _,
                delta: MouseScrollDelta::PixelDelta(zoom),
                phase: _,
            } => ControllerState {
                scroll: Vec2::new(zoom.x as f32, zoom.y as f32),
                ..default
            },
            _ => default,
        }
    }
}

pub struct Controller {
    state: ControllerState,
    camera: Camera,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
        }
    }

    pub fn event(&mut self, event: WindowEvent) {
        if let WindowEvent::Resized(size) = event {
            self.camera.aspect(size);
        }

        self.state = self.state.update(event);

        let rotation_speed = 0.01;
        let pan_speed = 0.1;
        let zoom_speed = 0.1;

        if self.state.left {
            self.camera.rotate(
                -self.state.delta.x * rotation_speed,
                -self.state.delta.y * rotation_speed,
            );
        }

        if self.state.right {
            self.camera.pan(
                self.state.delta.x * pan_speed,
                -self.state.delta.y * pan_speed,
            );
        }

        self.camera.zoom(-self.state.scroll.y * zoom_speed);
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn update(&mut self, dt: f32) {
        self.camera.update(dt);
    }
}
