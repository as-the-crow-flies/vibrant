use glam::Vec2;

use super::event::{Event, Key, MouseButton};

#[derive(Debug, Default, Clone, Copy)]
pub struct ControllerState {
    pub pressed: bool,
    pub left: bool,
    pub right: bool,
    pub position: Vec2,
    pub delta: Vec2,
    pub scroll: Vec2,
    pub size: Vec2,
    pub shift: bool,
}

impl ControllerState {
    pub fn relative_delta(&self) -> Vec2 {
        self.delta / self.size
    }

    pub fn update(&self, event: Event) -> Self {
        let default = ControllerState {
            delta: Vec2::default(),
            scroll: Vec2::default(),
            ..self.clone()
        };

        match event {
            Event::Resized(size) => ControllerState { size, ..default },
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
            Event::MouseReleased(MouseButton::Right) => ControllerState {
                pressed: false,
                right: false,
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
            _ => default,
        }
    }
}
