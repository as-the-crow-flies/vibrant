pub mod camera;
pub mod event;
pub mod light;
pub mod settings;
pub mod state;

use camera::Camera;
use egui::{FontId, Layout, RichText, Slider};
use event::Event;
use light::Light;
use settings::Settings;
use state::ControllerState;

use crate::{loader::AssetLoader, loader::Tractogram};

pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    settings: Settings,

    show_side_panel: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            settings: Settings::new(),

            show_side_panel: false,
        }
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);
        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(&mut self, ctx: &egui::Context, dt: f32) {
        egui::TopBottomPanel::top("TopBottomPanel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("⚙ settings").clicked() {
                    self.show_side_panel = !self.show_side_panel;
                }

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

        egui::SidePanel::left("SidePanel").show_animated(ctx, self.show_side_panel, |ui| {
            ui.add(
                Slider::new(&mut self.settings.ambient_occlusion_samples, 8..=100)
                    .text("Ambient Occlusion Samples"),
            );
            ui.add(
                Slider::new(&mut self.settings.streamline_radius, 0.01..=1.0)
                    .logarithmic(true)
                    .text("Streamline Radius"),
            );

            ui.add(
                Slider::new(&mut self.settings.direct_light, 0.0..=1.0)
                    .text("Direct Light vs Ambient Light"),
            );
        });
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn light(&self) -> &Light {
        &self.light
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }
}
