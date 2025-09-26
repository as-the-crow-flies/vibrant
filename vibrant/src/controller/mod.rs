pub mod camera;
pub mod event;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;

use camera::Camera;
use egui::{ComboBox, FontId, Layout, RichText, Slider};
use event::Event;
use light::Light;
use settings::{LineRenderMode, Settings};
use state::ControllerState;
use winit::dpi::PhysicalSize;

use crate::{
    controller::{
        segment::Segment,
        settings::{LineDisplayMode, LineVoxelizationMode},
    },
    file::File,
};

#[derive(Debug)]
pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    segment: Segment,
    settings: Settings,

    show_side_panel: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            segment: Segment::new(),
            settings: Settings::new(),

            show_side_panel: false,
        }
    }

    pub fn from_settings(settings: &Settings) -> Self {
        let mut controller = Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            segment: Segment::new(),
            settings: settings.clone(),
            show_side_panel: false,
        };

        controller.event(Event::Resized(settings.width, settings.height));

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
            ComboBox::from_label("Render Mode")
                .selected_text(format!("{:?}", self.settings.render))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.render,
                        LineRenderMode::RayTracing,
                        "RayTracing",
                    );
                    ui.selectable_value(
                        &mut self.settings.render,
                        LineRenderMode::RayTracingQuantized,
                        "RayTracingQuantized",
                    );
                    ui.selectable_value(
                        &mut self.settings.render,
                        LineRenderMode::RasterizationOrderCorrecting,
                        "RasterizationOrderCorrecting",
                    );
                    ui.selectable_value(
                        &mut self.settings.render,
                        LineRenderMode::Rasterization,
                        "Rasterization",
                    );
                });

            ComboBox::from_label("Display Mode")
                .selected_text(format!("{:?}", self.settings.display))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.display,
                        LineDisplayMode::Geometry,
                        "Geometry",
                    );
                    ui.selectable_value(
                        &mut self.settings.display,
                        LineDisplayMode::Volume,
                        "Volume",
                    );
                });

            ComboBox::from_label("Voxelization Mode")
                .selected_text(format!("{:?}", self.settings.voxelization))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.voxelization,
                        LineVoxelizationMode::Tube,
                        "Tube",
                    );
                    ui.selectable_value(
                        &mut self.settings.voxelization,
                        LineVoxelizationMode::Box,
                        "Box",
                    );
                    ui.selectable_value(
                        &mut self.settings.voxelization,
                        LineVoxelizationMode::Line,
                        "Line",
                    );
                });

            ui.separator();
            ui.label("Resolutions");
            ui.separator();

            ComboBox::from_label("Volume")
                .selected_text(format!("{:?}", self.settings.volume))
                .show_ui(ui, |ui| {
                    for power in 5u32..10 {
                        ui.selectable_value(
                            &mut self.settings.volume,
                            2u32.pow(power),
                            format!("{}", 2u32.pow(power)),
                        );
                    }
                });

            ui.separator();
            ui.label("Appearance");
            ui.separator();

            ui.add(Slider::new(&mut self.settings.radius, 0.01..=1.0).text("Streamline Radius"));
            ui.add(Slider::new(&mut self.settings.lighting, 0.0..=1.0).text("Lighting"));
            ui.add(Slider::new(&mut self.settings.direct_light, 0.0..=1.0).text("Ambient/Shadow"));
            ui.add(Slider::new(&mut self.settings.tangent_color, 0.0..=1.0).text("Tangent Color"));
            ui.add(Slider::new(&mut self.settings.alpha, 0.01..=1.0).text("Alpha"));
            ui.add(Slider::new(&mut self.settings.smoothing, 0.0..=1.0).text("Smoothing"));
            ui.add(Slider::new(&mut self.settings.slice_count, 1..=64).text("Culling Slices"));
            ui.add(Slider::new(&mut self.settings.workgroups, 1..=64).text("# Workgroups"));

            ui.checkbox(&mut self.settings.culling, "Enable Culling");
        });
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn light(&self) -> &Light {
        &self.light
    }

    pub fn segment(&self) -> &Segment {
        &self.segment
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.settings.width = size.width;
        self.settings.height = size.height;
    }
}
