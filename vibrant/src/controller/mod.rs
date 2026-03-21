pub mod camera;
pub mod event;
pub mod light;
pub mod selection_volume;
pub mod settings;
pub mod state;

use std::time::Instant;

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
    asset::Asset,
    controller::{
        selection_volume::{SelectionVolume, SelectionVolumeEntry},
        settings::{LineDisplayMode, LineVoxelizationMode},
    },
    file::FileStage,
};

#[derive(Debug)]
pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    settings: Settings,
    time: Instant,

    show_left_side_panel: bool,
    show_right_side_panel: bool,
    show_selection_panel: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            settings: Settings::new(),
            time: Instant::now(),

            show_left_side_panel: false,
            show_right_side_panel: false,
            show_selection_panel: false,
        }
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);

        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(&mut self, ctx: &egui::Context, asset: &mut Asset, dt: f32) {
        // Auto-rotate camera
        if self.settings.auto_rotate {
            let speed_rad = self.settings.auto_rotate_speed.to_radians();
            self.camera.yaw += speed_rad * dt;
        }

        egui::TopBottomPanel::top("TopBottomPanel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.show_left_side_panel, "⚙ settings")
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
                    if ui.selectable_label(self.show_right_side_panel, "☰ layers").clicked() {
                        self.show_right_side_panel = !self.show_right_side_panel
                    }

                    if ui.selectable_label(self.show_selection_panel, "◈ selection").clicked() {
                        self.show_selection_panel = !self.show_selection_panel;
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

                egui::CentralPanel::default()
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
                        egui::ScrollArea::vertical().show(ui, |ui| {
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
                                Slider::new(&mut self.camera.fov, 0.1..=3.0).text("Field of View"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.radius, 0.01..=1.0)
                                    .text("Streamline Radius"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.lighting, 0.0..=1.0)
                                    .text("Lighting"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.direct_light, 0.0..=3.0)
                                    .text("Ambient/Shadow"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.tangent_color, 0.0..=2.0)
                                    .text("Tangent Color"),
                            );
                            ui.add(Slider::new(&mut self.settings.alpha, 0.01..=1.0).text("Alpha"));
                            ui.add(
                                Slider::new(&mut self.settings.smoothing, 0.0..=1.0)
                                    .text("Smoothing"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.crop_start, 0.0..=1.0)
                                    .text("Crop Start"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.crop_end, 0.0..=1.0)
                                    .text("Crop End"),
                            );

                            if ui
                                .add(
                                    Slider::new(&mut self.settings.crop_middle, 0.0..=0.5)
                                        .text("Crop Middle"),
                                )
                                .changed()
                            {
                                self.settings.crop_start = 0.5 - self.settings().crop_middle;
                                self.settings.crop_end = 0.5 + self.settings().crop_middle;
                            }

                            ui.add(
                                Slider::new(&mut self.settings.crop_x_start, -0.5..=0.5)
                                    .step_by(1.0 / self.settings.volume as f64)
                                    .text("Crop X Start"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.crop_x_end, -0.5..=0.5)
                                    .step_by(1.0 / self.settings.volume as f64)
                                    .text("Crop X End"),
                            );

                            ui.add(
                                Slider::new(&mut self.settings.crop_y_start, -0.5..=0.5)
                                    .step_by(1.0 / self.settings.volume as f64)
                                    .text("Crop Y Start"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.crop_y_end, -0.5..=0.5)
                                    .step_by(1.0 / self.settings.volume as f64)
                                    .text("Crop Y End"),
                            );

                            ui.add(
                                Slider::new(&mut self.settings.crop_z_start, -0.5..=0.5)
                                    .step_by(1.0 / self.settings.volume as f64)
                                    .text("Crop Z Start"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.crop_z_end, -0.5..=0.5)
                                    .step_by(1.0 / self.settings.volume as f64)
                                    .text("Crop Z End"),
                            );

                            ui.add(Slider::new(&mut self.settings.plane, 0.0..=1.0).text("Plane"));

                            ui.separator();
                            ui.checkbox(&mut self.settings.auto_rotate, "Auto-Rotate");
                            if self.settings.auto_rotate {
                                ui.add(
                                    Slider::new(&mut self.settings.auto_rotate_speed, 1.0..=360.0)
                                        .text("Rotation Speed (°/s)"),
                                );
                            }

                            ui.separator();
                            ui.add(
                                Slider::new(&mut self.settings.workgroups, 1..=128)
                                    .text("# Workgroups"),
                            );
                            ui.separator();
                            ui.label("Post Processing");
                            ui.separator();
                            ui.add(
                                Slider::new(&mut self.settings.blur_kernel_size, 1..=32)
                                    .text("Blur Kernel Size"),
                            );
                            ui.checkbox(&mut self.settings.bloom, "Bloom");
                            ui.add_enabled(
                                self.settings.bloom,
                                Slider::new(&mut self.settings.bloom_threshold, 0.0..=1.0)
                                    .text("Bloom Threshold"),
                            );
                            ui.add_enabled(
                                self.settings.bloom,
                                Slider::new(&mut self.settings.bloom_intensity, 0.0..=3.0)
                                    .text("Bloom Intensity"),
                            );
                            ui.add_enabled(
                                self.settings.bloom,
                                Slider::new(&mut self.settings.bloom_spread, 1.0..=5.0)
                                    .text("Bloom Spread"),
                            );
                            ui.separator();
                            ui.checkbox(&mut self.settings.depth_of_field, "Depth of Field");
                            ui.add_enabled(
                                self.settings.depth_of_field,
                                Slider::new(&mut self.settings.focal_distance, 0.01..=3.0)
                                    .text("Focal Distance"),
                            );
                            ui.add_enabled(
                                self.settings.depth_of_field,
                                Slider::new(&mut self.settings.aperture, 0.01..=5.0)
                                    .text("Aperture"),
                            );
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
                                        .body(|ui| {
                                            ui.add(
                                                Slider::new(&mut line.crop_start, 0.0..=1.0)
                                                    .text("Crop Start"),
                                            );
                                            ui.add(
                                                Slider::new(&mut line.crop_end, 0.0..=1.0)
                                                    .text("Crop End"),
                                            );
                                        });
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

        SidePanel::right("SidePanelSelection")
            .min_width(300.0)
            .show_animated(ctx, self.show_selection_panel, |ui| {
                CollapsingState::load_with_default_open(ui.ctx(), "SelectionVolumes".into(), true)
                    .show_header(ui, |ui| ui.heading("Selection Volumes"))
                    .body(|ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("+ Add").clicked() {
                                    self.settings
                                        .selection_volumes
                                        .push(SelectionVolumeEntry::default());
                                }
                                ui.checkbox(
                                    &mut self.settings.selection_match_all,
                                    "Match All Conditions",
                                );
                                ui.checkbox(&mut self.settings.extend_lines, "Extend Lines");
                            });

                            let mut to_remove: Option<usize> = None;
                            for (i, vol) in self.settings.selection_volumes.iter_mut().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ComboBox::from_id_salt(("sel_vol", i))
                                            .selected_text(format!("{:?}", vol.shape))
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut vol.shape,
                                                    SelectionVolume::None,
                                                    "None",
                                                );
                                                ui.selectable_value(
                                                    &mut vol.shape,
                                                    SelectionVolume::Box,
                                                    "Box",
                                                );
                                                ui.selectable_value(
                                                    &mut vol.shape,
                                                    SelectionVolume::Sphere,
                                                    "Sphere",
                                                );
                                            });
                                        if ui.button("✕").clicked() {
                                            to_remove = Some(i);
                                        }
                                    });
                                    ui.add(
                                        egui::Slider::new(&mut vol.scale, 0.0..=5.0).text("Scale"),
                                    );
                                    ui.add(
                                        egui::Slider::new(&mut vol.offset_x, -1.0..=1.0)
                                            .text("Offset X"),
                                    );
                                    ui.add(
                                        egui::Slider::new(&mut vol.offset_y, -1.0..=1.0)
                                            .text("Offset Y"),
                                    );
                                    ui.add(
                                        egui::Slider::new(&mut vol.offset_z, -1.0..=1.0)
                                            .text("Offset Z"),
                                    );
                                    ui.checkbox(&mut vol.negate, "Negate");
                                    ui.checkbox(&mut vol.highlight, "Highlight Volume");
                                });
                            }
                            if let Some(idx) = to_remove {
                                self.settings.selection_volumes.remove(idx);
                            }
                        });
                    });
            });
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn set_camera_distance(&mut self, distance: f32) {
        self.camera.set_distance(distance);
    }

    pub fn light(&self) -> &Light {
        &self.light
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.settings.width = size.width;
        self.settings.height = size.height;
    }

    pub fn time(&self) -> f32 {
        Instant::now().duration_since(self.time).as_secs_f32()
    }
}

fn ternary_checkbox(ui: &mut Ui, input: Option<bool>, text: &str) -> Option<bool> {
    let mut checked = input.unwrap_or_default();

    ui.toggle_value(&mut checked, text)
        .clicked()
        .then_some(checked)
}
