pub mod camera;
pub mod event;
pub mod light;
pub mod settings;
pub mod state;

use camera::Camera;
use egui::{ComboBox, FontId, Layout, RichText, Slider};
use event::Event;
use light::Light;
use settings::{DensitySetting, GeometrySetting, Settings, ShadingSetting};
use state::ControllerState;

use crate::file::File;

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
                    File::load();
                }

                if ui.button("📷 screenshot").clicked() {
                    File::save();
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
            ComboBox::from_label("Density")
                .selected_text(format!("{:?}", self.settings.density))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.density, DensitySetting::Add, "Add");
                    ui.selectable_value(&mut self.settings.density, DensitySetting::Or, "Or");
                });

            ComboBox::from_label("Geometry")
                .selected_text(format!("{:?}", self.settings.geometry))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.geometry,
                        GeometrySetting::LineHardware,
                        "LineHardware",
                    );
                    ui.selectable_value(
                        &mut self.settings.geometry,
                        GeometrySetting::LineSoftware,
                        "LineSoftware",
                    );
                    ui.selectable_value(&mut self.settings.geometry, GeometrySetting::Tube, "Tube");
                });

            ComboBox::from_label("Shading")
                .selected_text(format!("{:?}", self.settings.shading))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Tracing,
                        "Tracing",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Simple,
                        "Simple",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Density,
                        "Density",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Occlusion,
                        "Occlusion",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::GBuffer,
                        "GBuffer",
                    );
                });

            ui.checkbox(&mut self.settings.culling, "Culling");
            ui.checkbox(&mut self.settings.balancing, "Balancing");

            ui.add(
                Slider::new(&mut self.settings.streamline_radius, 0.01..=0.5)
                    .logarithmic(true)
                    .text("Streamline Radius"),
            );

            ui.add(
                Slider::new(&mut self.settings.direct_light, 0.0..=1.0)
                    .text("Direct Light vs Ambient Light"),
            );

            ui.add(
                Slider::new(&mut self.settings.shading_level, 0.0..=2.0).text("Shading Strength"),
            );

            ui.add(
                Slider::new(&mut self.settings.gradient_factor, 0.0..=1.0).text("Tangent Coloring"),
            );

            ui.add(Slider::new(&mut self.settings.cull_level, 1.0..=256.0).text("Cull Level"));
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
