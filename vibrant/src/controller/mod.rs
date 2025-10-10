pub mod camera;
pub mod event;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;

use camera::Camera;
use egui::{ComboBox, FontId, Frame, Layout, Margin, RichText, Slider};
use event::Event;
use light::Light;
use settings::Settings;
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
                if ui
                    .button("⚙ settings")
                    .on_hover_text("Open settings panel")
                    .clicked()
                {
                    self.show_side_panel = !self.show_side_panel;
                }

                if ui
                    .button("📂 open")
                    .on_hover_text("Open .tck/.obj files")
                    .clicked()
                {
                    File::load();
                }

                #[cfg(not(target_arch = "wasm32"))]
                if ui
                    .button("📷 screenshot")
                    .on_hover_text("Take screenshot with transparent background")
                    .clicked()
                {
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
            egui::TopBottomPanel::top("top_panel")
                .frame(Frame {
                    outer_margin: Margin {
                        left: 5,
                        right: 5,
                        top: 5,
                        bottom: 10,
                    },
                    inner_margin: Margin::ZERO,
                    ..Default::default()
                })
                .show_inside(ui, |ui| {
                    ui.heading("Rendering");
                    ui.separator();

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

                    ComboBox::from_label("Voxel Resolution")
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

                    ui.add(
                        Slider::new(&mut self.settings.radius, 0.01..=1.0)
                            .text("Streamline Radius"),
                    );
                    ui.add(Slider::new(&mut self.settings.lighting, 0.0..=1.0).text("Lighting"));
                    ui.add(
                        Slider::new(&mut self.settings.direct_light, 0.0..=1.0)
                            .text("Ambient/Shadow"),
                    );
                    ui.add(
                        Slider::new(&mut self.settings.tangent_color, 0.0..=1.0)
                            .text("Tangent Color"),
                    );
                    ui.add(Slider::new(&mut self.settings.alpha, 0.01..=1.0).text("Alpha"));
                    ui.add(Slider::new(&mut self.settings.smoothing, 0.0..=1.0).text("Smoothing"));
                    ui.add(
                        Slider::new(&mut self.settings.workgroups, 1..=128).text("# Workgroups"),
                    );
                });

            egui::TopBottomPanel::bottom("bottom_panel")
                .frame(Frame {
                    outer_margin: Margin {
                        left: 5,
                        right: 5,
                        top: 5,
                        bottom: 10,
                    },
                    inner_margin: Margin::ZERO,
                    ..Default::default()
                })
                .show_inside(ui, |ui| {
                    ui.heading("Controls");
                    ui.separator();

                    egui::Grid::new("my_grid")
                        .min_col_width(100.0)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Rotate Camera");
                            ui.label("Left Mouse Button");
                            ui.end_row();

                            ui.label("Pan Camera");
                            ui.label("Right Mouse Button");
                            ui.end_row();

                            ui.label("Zoom Camera");
                            ui.label("Mouse Wheel");
                            ui.end_row();

                            ui.label("Reset Camera");
                            ui.label("Backspace");
                            ui.end_row();

                            ui.label("Rotate Light");
                            ui.label("Shift + Left Mouse Button");
                            ui.end_row();
                        });
                });
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
