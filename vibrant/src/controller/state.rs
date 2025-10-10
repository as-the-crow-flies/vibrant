use glam::Vec2;

use super::event::{Event, Key, MouseButton};

#[derive(Debug, Default, Clone, Copy)]
pub struct ControllerState {
    pub width: u32,
    pub height: u32,
    pub volume: u32,
    pub pressed: bool,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub position: Vec2,
    pub delta: Vec2,
    pub scroll: Vec2,
    pub shift: bool,
    pub backspace: bool,
}

impl ControllerState {
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32)
    }

    pub fn relative_delta(&self) -> Vec2 {
        self.delta / self.size()
    }

    pub fn update(&self, event: Event) -> Self {
        let default = ControllerState {
            delta: Vec2::default(),
            scroll: Vec2::default(),
            ..self.clone()
        };

        match event {
            Event::Resized(width, height) => ControllerState {
                width,
                height,
                ..default
            },
            Event::MouseMoved(position) => ControllerState {
                position,
                delta: position - self.position,
                ..default
            },
            Event::MousePressed(MouseButton::Left) => ControllerState {
                pressed: true,
                left: true,
                ..default
            },
            Event::MouseReleased(MouseButton::Left) => ControllerState {
                pressed: false,
                left: false,
                ..default
            },
            Event::MousePressed(MouseButton::Right) => ControllerState {
                pressed: true,
                right: true,
                ..default
            },
            Event::MousePressed(MouseButton::Middle) => ControllerState {
                pressed: true,
                middle: true,
                ..default
            },
            Event::MouseReleased(MouseButton::Right) => ControllerState {
                pressed: false,
                right: false,
                ..default
            },
            Event::MouseReleased(MouseButton::Middle) => ControllerState {
                pressed: false,
                middle: false,
                ..default
            },
            Event::MouseWheel(scroll) => ControllerState { scroll, ..default },
            Event::KeyPressed(Key::Shift) => ControllerState {
                shift: true,
                ..default
            },
            Event::KeyReleased(Key::Shift) => ControllerState {
                shift: false,
                ..default
            },
            Event::KeyPressed(Key::Backspace) => ControllerState {
                backspace: true,
                ..default
            },
            Event::KeyReleased(Key::Backspace) => ControllerState {
                backspace: false,
                ..default
            },
        }
    }
}
