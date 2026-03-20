pub mod camera;
pub mod event;
pub mod light;
pub mod segment;
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
        segment::Segment,
        settings::{AntiAliasingMode, LineDisplayMode, LineVoxelizationMode},
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
    time: Instant,

    show_left_side_panel: bool,
    show_right_side_panel: bool,

    prefer_hdr_output: bool,
    output_hdr_supported: bool,
    output_hdr: bool,
    output_format: String,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            segment: Segment::new(),
            settings: Settings::new(),
            time: Instant::now(),

            show_left_side_panel: false,
            show_right_side_panel: false,
            prefer_hdr_output: false,
            output_hdr_supported: false,
            output_hdr: false,
            output_format: "Unknown".to_string(),
        }
    }

    pub fn set_output_mode(
        &mut self,
        output_hdr: bool,
        output_format: String,
        output_hdr_supported: bool,
    ) {
        self.output_hdr = output_hdr;
        self.output_format = output_format;
        self.output_hdr_supported = output_hdr_supported;

        if !output_hdr_supported {
            self.prefer_hdr_output = false;
        }
    }

    pub fn prefer_hdr_output(&self) -> bool {
        self.prefer_hdr_output
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);

        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(&mut self, ctx: &egui::Context, asset: &mut Asset, _dt: f32) {
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

                        ui.add(Slider::new(&mut self.camera.fov, 0.1..=3.0));
                        ui.add(
                            Slider::new(&mut self.settings.radius, 0.01..=1.0)
                                .text("Streamline Radius"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.lighting, 0.0..=1.0).text("Lighting"),
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
                            Slider::new(&mut self.settings.smoothing, 0.0..=1.0).text("Smoothing"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.crop_start, 0.0..=1.0)
                                .text("Crop Start"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.crop_end, 0.0..=1.0).text("Crop End"),
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

                        ui.add(Slider::new(&mut self.settings.plane, 0.0..=1.0).text("plane"));

                        ui.add(
                            Slider::new(&mut self.settings.workgroups, 1..=128)
                                .text("# Workgroups"),
                        );

                        // Bloom settings
                        ui.checkbox(&mut self.settings.bloom_enabled, "Enable Bloom");
                        ui.add(
                            Slider::new(&mut self.settings.bloom_threshold, 0.0..=2.0)
                                .text("Bloom Threshold"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.bloom_soft_knee, 0.0..=1.0)
                                .text("Bloom Soft Knee"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.bloom_intensity, 0.0..=5.0)
                                .text("Bloom Intensity")
                        );

                        // anti-aliasing mode selector
                        ui.separator();
                        ui.label("Anti-Aliasing");
                        ComboBox::from_label("AA Mode")
                            .selected_text(format!("{:?}", self.settings.aa_mode))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.aa_mode,
                                    AntiAliasingMode::Off,
                                    "Off",
                                );
                                ui.selectable_value(
                                    &mut self.settings.aa_mode,
                                    AntiAliasingMode::SMAA,
                                    "SMAA",
                                );
                                ui.selectable_value(
                                    &mut self.settings.aa_mode,
                                    AntiAliasingMode::TAA,
                                    "TAA",
                                );
                            });

                        // SMAA tuning parameters — only shown when SMAA is active.
                        if self.settings.aa_mode == AntiAliasingMode::SMAA {
                            ui.add(
                                Slider::new(&mut self.settings.smaa_threshold, 0.05..=0.20)
                                    .text("Edge Threshold")
                            );
                            let mut steps = self.settings.smaa_max_search_steps as i32;
                            if ui.add(
                                Slider::new(&mut steps, 4..=32)
                                    .text("Max Search Steps")
                            ).changed() {
                                self.settings.smaa_max_search_steps = steps as u32;
                            }
                        }

                        // TAA tuning parameters — only shown when TAA is active.
                        if self.settings.aa_mode == AntiAliasingMode::TAA {
                            ui.add(
                                Slider::new(&mut self.settings.taa_blend_factor, 0.05..=0.30)
                                    .text("Blend Factor")
                            );
                            ui.add(
                                Slider::new(&mut self.settings.taa_clamp_sigma, 0.5..=2.0)
                                    .text("Clamp Sigma")
                            );
                        }

                        if ui
                            .add(Slider::new(&mut self.settings.render_scale, 0.25..=2.0)
                                .text("Render Scale"))
                            .changed()
                        {
                            self.settings.update_render_size();
                        }

                        ui.label(format!(
                            "3D Render Resolution: {} x {}",
                            self.settings.render_width, self.settings.render_height
                        ));


                        ui.separator();
                        ui.heading("HDR Output");

                        if self.output_hdr_supported {
                            ui.checkbox(&mut self.prefer_hdr_output, "Enable HDR Output");
                        } else {
                            let mut enabled = false;
                            ui.add_enabled(
                                false,
                                egui::Checkbox::new(&mut enabled, "Enable HDR Output"),
                            );
                            ui.label("HDR unavailable on current display/surface");
                        }

                        ui.label(format!(
                            "Current: {}",
                            if self.output_hdr { "HDR" } else { "SDR" }
                        ));
                        ui.label(format!(
                            "Requested: {}",
                            if self.prefer_hdr_output { "HDR" } else { "SDR" }
                        ));
                        ui.label(format!("Format: {}", self.output_format));

                        // Key HDR mapping controls for SDR UI placement on HDR displays.
                        ui.add(
                            Slider::new(&mut self.settings.hdr_paper_white_nits, 80.0..=400.0)
                                .text("Paper White (nits)"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.hdr_peak_nits, 400.0..=2000.0)
                                .text("Peak Brightness (nits)"),
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
        self.settings.update_render_size();
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
