use glam::Vec2;

pub enum MouseButton {
    Left,
    Right,
}

pub enum Key {
    Shift,
}

pub enum Event {
    Resized(u32, u32),
    MouseMoved(Vec2),
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
    MouseWheel(Vec2),
    KeyPressed(Key),
    KeyReleased(Key),
}
