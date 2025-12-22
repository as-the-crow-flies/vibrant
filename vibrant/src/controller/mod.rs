pub mod camera;
pub mod event;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;

use camera::Camera;
use egui::{
    collapsing_header::CollapsingState, Align, ComboBox, Frame, Layout, Margin, ScrollArea,
    SidePanel, Slider, Ui,
};
use event::Event;
use itertools::Itertools;
use light::Light;
use settings::Settings;
use state::ControllerState;
use winit::dpi::PhysicalSize;

use crate::{
    asset::{line::LineBuffer, Asset},
    controller::{
        segment::Segment,
        settings::{LineDisplayMode, LineVoxelizationMode},
    },
    file::FileStage,
};

#[derive(Debug)]
pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    segment: Segment,
    settings: Settings,

    show_left_side_panel: bool,
    show_right_side_panel: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            segment: Segment::new(),
            settings: Settings::new(),

            show_left_side_panel: false,
            show_right_side_panel: true,
        }
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);

        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(&mut self, ctx: &egui::Context, asset: &mut Asset) {
        egui::TopBottomPanel::top("TopBottomPanel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("⚙ settings")
                    .on_hover_text("Open settings panel")
                    .clicked()
                {
                    self.show_left_side_panel = !self.show_left_side_panel;
                }

                #[cfg(not(target_arch = "wasm32"))]
                if ui
                    .button("📷 screenshot")
                    .on_hover_text("Take screenshot with transparent background")
                    .clicked()
                {
                    FileStage::save();
                }

                ui.take_available_width();

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("☰ layers").clicked() {
                        self.show_right_side_panel = !self.show_right_side_panel
                    }

                    if ui
                        .button("📂 open")
                        .on_hover_text("Open .tck/.obj files")
                        .clicked()
                    {
                        FileStage::load();
                    }
                });
            });
        });

        egui::SidePanel::left("SidePanelLeft").show_animated(
            ctx,
            self.show_left_side_panel,
            |ui| {
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
                        ui.add(
                            Slider::new(&mut self.settings.lighting, 0.0..=1.0).text("Lighting"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.direct_light, 0.0..=1.0)
                                .text("Ambient/Shadow"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.tangent_color, 0.0..=1.0)
                                .text("Tangent Color"),
                        );
                        ui.add(Slider::new(&mut self.settings.alpha, 0.01..=1.0).text("Alpha"));
                        ui.add(
                            Slider::new(&mut self.settings.smoothing, 0.0..=1.0).text("Smoothing"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.workgroups, 1..=128)
                                .text("# Workgroups"),
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
            },
        );

        SidePanel::right("SidePanelRight")
            .min_width(300.0)
            .show_animated(ctx, self.show_right_side_panel, |ui| {
                CollapsingState::load_with_default_open(ui.ctx(), "Tractography".into(), false)
                    .show_header(ui, |ui| ui.heading("Tractography"))
                    .body(|ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            if let Some(lines) = &mut asset.line {
                                lines.settings_global().selected = lines
                                    .settings()
                                    .iter()
                                    .map(|settings| settings.selected)
                                    .all_equal_value()
                                    .ok();

                                lines.settings_global().visible = lines
                                    .settings()
                                    .iter()
                                    .map(|settings| settings.visible)
                                    .all_equal_value()
                                    .ok();

                                CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    "Line".into(),
                                    false,
                                )
                                .show_header(ui, |ui| {
                                    if let Some(visible) =
                                        ternary_checkbox(ui, lines.settings_global().visible, "👁")
                                    {
                                        lines.settings_global().visible = Some(visible);

                                        for line in lines.settings() {
                                            line.visible = visible;
                                        }
                                    }

                                    if let Some(color_visible) = ternary_checkbox(
                                        ui,
                                        Some(lines.settings_global().color_visible),
                                        "   🎨   ",
                                    ) {
                                        lines.settings_global().color_visible = color_visible;

                                        for line in lines.settings() {
                                            line.color_visible = color_visible
                                        }
                                    }
                                })
                                .body(|_| {});

                                for line in lines.settings() {
                                    let id = ui.make_persistent_id(&line.name);
                                    CollapsingState::load_with_default_open(ui.ctx(), id, false)
                                        .show_header(ui, |ui| {
                                            ui.toggle_value(&mut line.visible, "👁");
                                            ui.color_edit_button_srgb(&mut line.color);
                                            ui.label(&line.name);
                                        })
                                        .body(|_| {});
                                }
                            }
                        });
                    });

                CollapsingState::load_with_default_open(ui.ctx(), "Volumes".into(), false)
                    .show_header(ui, |ui| ui.heading("Volumes"))
                    .body(|ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            for volume in &mut asset.volumes {
                                let id = ui.make_persistent_id(&volume.name());
                                CollapsingState::load_with_default_open(ui.ctx(), id, false)
                                    .show_header(ui, |ui| {
                                        ui.label(volume.name());
                                    })
                                    .body(|_| {});
                            }
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

fn ternary_checkbox(ui: &mut Ui, input: Option<bool>, text: &str) -> Option<bool> {
    let mut checked = input.unwrap_or_default();

    ui.toggle_value(&mut checked, text)
        .clicked()
        .then_some(checked)
}
