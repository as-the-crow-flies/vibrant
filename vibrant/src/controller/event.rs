use glam::Vec2;

pub enum MouseButton {
    Left,
    Right,
}

pub enum Event {
    Resized(Vec2),
    MouseMoved(Vec2),
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
    MouseWheel(Vec2),
}
