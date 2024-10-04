pub mod camera;
pub mod event;
pub mod state;

use camera::Camera;
use egui::{FontId, Layout, RichText};
use event::Event;
use state::ControllerState;

use crate::{loader::AssetLoader, loader::Tractogram};

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
        self.state = self.state.update(event);
        self.camera.update(&self.state);
    }

    pub fn ui(&mut self, ctx: &egui::Context, dt: f32) {
        egui::TopBottomPanel::top("TopBottomPanel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 open").clicked() {
                    Tractogram::file_dialog(|tractogram| {
                        AssetLoader::publish_tractogram(tractogram);
                    });
                }

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:3.0} fps ({:3.0} ms)", 1.0 / dt, 1000.0 * dt))
                            .font(FontId::monospace(12.0)),
                    );
                })
            });
        });
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }
}
