pub mod camera;
pub mod event;

use camera::Camera;
use egui_wgpu::winit;
use event::{Event, MouseButton};
use glam::Vec2;

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
    fn update(&self, event: Event) -> Self {
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

    pub fn event(&mut self, event: Event) {
        if let Event::Resized(size) = event {
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

    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("VIBRANT")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
            .show(ctx, |ui| {
                ui.label("Label!");

                if ui.button("Button!").clicked() {
                    println!("boom!")
                }
            });
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn update(&mut self, dt: f32) {
        self.camera.update(dt);
    }
}
