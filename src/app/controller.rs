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
    size: Vec2,
}

impl ControllerState {
    fn relative_delta(&self) -> Vec2 {
        self.delta / self.size
    }
}

impl ControllerState {
    fn update(&self, event: WindowEvent) -> Self {
        let default = ControllerState {
            delta: Vec2::default(),
            scroll: Vec2::default(),
            ..self.clone()
        };

        match event {
            WindowEvent::Resized(size) => ControllerState {
                size: Vec2::new(size.width as f32, size.height as f32),
                ..default
            },
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

        if self.state.left {
            let rotation = self.state.relative_delta() * 10.0;
            self.camera.rotate(-rotation.x, -rotation.y);
        }

        if self.state.right {
            let pan = self.state.relative_delta();
            self.camera.pan(pan.x, -pan.y);
        }

        self.camera.zoom(-self.state.scroll.y * 0.05);
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn update(&mut self, dt: f32) {
        self.camera.update(dt);
    }
}
