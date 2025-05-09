pub mod camera;
pub mod event;
pub mod light;
pub mod settings;
pub mod state;

use camera::Camera;
use egui::{ComboBox, FontId, Layout, RichText, Slider};
use event::Event;
use light::Light;
use settings::{GeometrySetting, Settings, ShadingSetting};
use state::ControllerState;
use winit::dpi::PhysicalSize;

use crate::file::File;

#[derive(Debug)]
pub struct Controller {
    width: u32,
    height: u32,
    volume: u32,
    tile: u32,
    layers: u32,
    state: ControllerState,
    camera: Camera,
    light: Light,
    settings: Settings,

    show_side_panel: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            width: 1920,
            height: 1080,
            volume: 256,
            tile: 8,
            layers: 16,
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            settings: Settings::new(),

            show_side_panel: false,
        }
    }

    pub fn test(width: u32, height: u32, volume: u32, tile: u32, layers: u32) -> Self {
        let mut controller = Self {
            width,
            height,
            volume,
            tile,
            layers,
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            settings: Settings::new(),
            show_side_panel: false,
        };

        controller.event(Event::Resized(width, height));

        controller
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
            ComboBox::from_label("Geometry")
                .selected_text(format!("{:?}", self.settings.geometry))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.geometry, GeometrySetting::Line, "Line");
                    ui.selectable_value(&mut self.settings.geometry, GeometrySetting::Tube, "Tube");
                    ui.selectable_value(
                        &mut self.settings.geometry,
                        GeometrySetting::Transparency,
                        "Transparency",
                    );
                });

            ComboBox::from_label("Shading")
                .selected_text(format!("{:?}", self.settings.shading))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Simple,
                        "Simple",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::GBuffer,
                        "GBuffer",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Density,
                        "Density",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Culling,
                        "Culling",
                    );
                    ui.selectable_value(
                        &mut self.settings.shading,
                        ShadingSetting::Tracing,
                        "Tracing",
                    );
                });

            ui.separator();
            ui.label("Resolutions");
            ui.separator();

            ComboBox::from_label("Volume")
                .selected_text(format!("{:?}", self.volume))
                .show_ui(ui, |ui| {
                    for power in 5u32..10 {
                        ui.selectable_value(
                            &mut self.volume,
                            2u32.pow(power),
                            format!("{}", 2u32.pow(power)),
                        );
                    }
                });

            ComboBox::from_label("Tile")
                .selected_text(format!("{:?}", self.tile))
                .show_ui(ui, |ui| {
                    for power in 1u32..6 {
                        ui.selectable_value(
                            &mut self.tile,
                            2u32.pow(power),
                            format!("{}", 2u32.pow(power)),
                        );
                    }
                });

            ComboBox::from_label("Layers")
                .selected_text(format!("{:?}", self.layers))
                .show_ui(ui, |ui| {
                    for power in 3u32..7 {
                        ui.selectable_value(
                            &mut self.layers,
                            2u32.pow(power),
                            format!("{}", 2u32.pow(power)),
                        );
                    }
                });

            ui.separator();
            ui.label("Appearance");
            ui.separator();

            ui.add(
                Slider::new(&mut self.settings.streamline_radius, 0.1..=1.0)
                    .text("Streamline Radius"),
            );

            ui.add(Slider::new(&mut self.settings.direct_light, 0.0..=1.0).text("Direct Light"));

            ui.add(
                Slider::new(&mut self.settings.alpha, 0.0001..=1.0)
                    .logarithmic(true)
                    .text("Alpha"),
            );

            ui.add(
                Slider::new(&mut self.settings.culling_threshold, 0.0..=10.0)
                    .text("Culling Threshold"),
            );

            ui.add(Slider::new(&mut self.settings.level, 0.0..=16.0).text("Level"));
            ui.add(Slider::new(&mut self.settings.layer, 0..=self.layers).text("Layer"));
            ui.checkbox(&mut self.settings.quality, "Quality");
        });
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn volume(&self) -> u32 {
        self.volume
    }

    pub fn tile(&self) -> u32 {
        self.tile
    }

    pub fn layers(&self) -> u32 {
        self.layers
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

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.width = size.width;
        self.height = size.height;
    }
}
