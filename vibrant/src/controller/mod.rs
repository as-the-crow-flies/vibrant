pub mod animation;
pub mod camera;
pub mod event;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;

use animation::Animation;
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
    pre_ssaa_render_scale: f32, // render_scale saved before entering SSAA mode, restored on exit
    pub animation: Option<Animation>,
    pub animation_just_finished: bool,
    // last-frame GPU pass timings set by the renderer each frame.
    gpu_profile: Vec<(String, f32)>,
    // whether the GPU timings panel is currently open in the UI.
    pub gpu_profile_open: bool,
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
            animation: None,
            animation_just_finished: false,
            prefer_hdr_output: false,
            output_hdr_supported: false,
            output_hdr: false,
            output_format: "Unknown".to_string(),
            pre_ssaa_render_scale: 1.0,
            gpu_profile: Vec::new(),
            gpu_profile_open: false,
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

    // update GPU rendering time
    pub fn set_gpu_profile(&mut self, results: Vec<(&str, f32)>) {
        self.gpu_profile = results
            .into_iter()
            .map(|(label, ms)| (label.to_string(), ms))
            .collect();
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);

        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(&mut self, ctx: &egui::Context, asset: &mut Asset, _dt: f32) {
        self.camera.tick();
        self.resolve_adaptive_aa();

        if self.settings.foveated {
            let mouse = self.mouse_ndc();
            self.settings.foveated_mouse_x = mouse.x;
            self.settings.foveated_mouse_y = mouse.y;
        }
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

                let record_label = if self.settings.recording {
                    "⏹ stop recording"
                } else {
                    "⏺ record"
                };
                if ui
                    .button(record_label)
                    .on_hover_text("Record video to recording.mp4")
                    .clicked()
                {
                    self.settings.recording = !self.settings.recording;
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
                // Bottom panel MUST be added first so egui reserves its space
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

                // ScrollArea fills all remaining space above the bottom panel
                ScrollArea::new([false, true])
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(5.0);
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

                        ui.separator();
                        ui.label("Bloom");
                        ui.separator();

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
                                .text("Bloom Intensity"),
                        );

                        ui.separator();
                        ui.label("Anti-Aliasing");
                        ui.separator();

                        ui.label(format!(
                            "Camera: {} (v={:.6})",
                            self.camera.motion_level(),
                            self.camera.velocity()
                        ));
                        let prev_aa_mode = self.settings.aa_mode;
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
                                    AntiAliasingMode::SSAA,
                                    "SSAA",
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
                                ui.selectable_value(
                                    &mut self.settings.aa_mode,
                                    AntiAliasingMode::Adaptive,
                                    "Adaptive",
                                );
                            });

                        if prev_aa_mode != self.settings.aa_mode {
                            let enters_scaled = matches!(
                                self.settings.aa_mode,
                                AntiAliasingMode::SSAA | AntiAliasingMode::Adaptive
                            );
                            let leaves_scaled = matches!(
                                prev_aa_mode,
                                AntiAliasingMode::SSAA | AntiAliasingMode::Adaptive
                            );
                            if enters_scaled && !leaves_scaled {
                                self.pre_ssaa_render_scale = self.settings.render_scale;
                                if self.settings.aa_mode == AntiAliasingMode::SSAA {
                                    self.settings.render_scale = std::f32::consts::SQRT_2;
                                    self.settings.update_render_size();
                                }
                            } else if leaves_scaled && !enters_scaled {
                                self.settings.render_scale = self.pre_ssaa_render_scale;
                                self.settings.update_render_size();
                            }
                        }

                        if self.settings.aa_mode == AntiAliasingMode::Adaptive {
                            ui.label(format!(
                                "Current AA Strategy: {:?}",
                                self.settings.effective_aa_mode
                            ));
                        }

                        if self.settings.aa_mode == AntiAliasingMode::SMAA {
                            ui.add(
                                Slider::new(&mut self.settings.smaa_threshold, 0.05..=0.20)
                                    .text("Edge Threshold"),
                            );
                            let mut steps = self.settings.smaa_max_search_steps as i32;
                            if ui
                                .add(Slider::new(&mut steps, 4..=32).text("Max Search Steps"))
                                .changed()
                            {
                                self.settings.smaa_max_search_steps = steps as u32;
                            }
                        }

                        if self.settings.aa_mode == AntiAliasingMode::TAA {
                            ui.add(
                                Slider::new(&mut self.settings.taa_blend_factor, 0.05..=0.30)
                                    .text("Blend Factor"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.taa_clamp_sigma, 0.5..=2.0)
                                    .text("Clamp Sigma"),
                            );
                        }

                        if self.settings.aa_mode == AntiAliasingMode::SSAA {
                            ui.horizontal(|ui| {
                                if ui.button("2x (√2)").clicked() {
                                    self.settings.render_scale = std::f32::consts::SQRT_2;
                                    self.settings.update_render_size();
                                }
                                if ui.button("4x (2.0)").clicked() {
                                    self.settings.render_scale = 2.0;
                                    self.settings.update_render_size();
                                }
                            });
                            if ui
                                .add(
                                    Slider::new(&mut self.settings.render_scale, 1.0..=2.0)
                                        .text("SSAA Scale"),
                                )
                                .changed()
                            {
                                self.settings.update_render_size();
                            }
                        }

                        if !matches!(
                            self.settings.aa_mode,
                            AntiAliasingMode::SSAA | AntiAliasingMode::Adaptive
                        ) {
                            if ui
                                .add(
                                    Slider::new(&mut self.settings.render_scale, 0.25..=2.0)
                                        .text("Render Scale"),
                                )
                                .changed()
                            {
                                self.settings.update_render_size();
                            }
                        }

                        ui.label(format!(
                            "3D Render Resolution: {} x {}",
                            self.settings.render_width, self.settings.render_height
                        ));

                        // GPU pass timings panel
                        {
                            let id = ui.make_persistent_id("gpu_pass_timings");
                            self.gpu_profile_open = CollapsingState::load(ui.ctx(), id)
                                .map(|s| s.is_open())
                                .unwrap_or(false);

                            ui.separator();
                            CollapsingState::load_with_default_open(ui.ctx(), id, false)
                                .show_header(ui, |ui| {
                                    ui.label("GPU Pass Timings");
                                })
                                .body(|ui| {
                                    let total: f32 =
                                        self.gpu_profile.iter().map(|(_, ms)| ms).sum();
                                    for (label, ms) in &self.gpu_profile {
                                        ui.label(format!("{:<12} {:>6.2} ms", label, ms));
                                    }
                                    ui.separator();
                                    ui.label(format!("{:<12} {:>6.2} ms", "total", total));
                                });
                        }

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

                        ui.add(
                            Slider::new(&mut self.settings.hdr_paper_white_nits, 80.0..=400.0)
                                .text("Paper White (nits)"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.hdr_peak_nits, 400.0..=2000.0)
                                .text("Peak Brightness (nits)"),
                        );

                        // Foveated rendering
                        ui.separator();
                        ui.heading("Foveated Rendering");

                        if ui
                            .checkbox(&mut self.settings.foveated, "Enable Foveated")
                            .changed()
                        {
                            self.settings.update_render_size();
                        }

                        if self.settings.foveated {
                            if ui
                                .add(
                                    Slider::new(
                                        &mut self.settings.foveated_peripheral_scale,
                                        0.25..=0.75,
                                    )
                                    .text("Peripheral Scale"),
                                )
                                .changed()
                            {
                                self.settings.update_render_size();
                            }
                            ui.add(
                                Slider::new(&mut self.settings.foveated_focus_radius, 0.05..=0.5)
                                    .text("Focus Radius"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.foveated_focus_scale, 0.5..=4.0)
                                    .text("Focus Scale"),
                            );
                            ui.add(
                                Slider::new(&mut self.settings.foveated_blend_width, 0.01..=0.15)
                                    .text("Blend Width"),
                            );

                            ui.label(format!(
                                "Focus: ({:.2}, {:.2})",
                                self.settings.foveated_mouse_x,
                                self.settings.foveated_mouse_y
                            ));
                            ui.label(format!(
                                "Peripheral: {} x {}",
                                self.settings.peripheral_width(),
                                self.settings.peripheral_height()
                            ));
                            ui.label(format!(
                                "Focus: {} x {}",
                                self.settings.focus_width(),
                                self.settings.focus_height()
                            ));
                        }

                        ui.separator();
                        ui.label("Animation");
                        ui.separator();

                        // Framerate options
                        ui.horizontal(|ui| {
                            ui.label("Recording FPS:");
                            ComboBox::from_id_salt("recording_fps")
                                .selected_text(format!("{} fps", self.settings.recording_fps))
                                .show_ui(ui, |ui| {
                                    for fps in [25u32, 30, 60, 120, 240] {
                                        ui.selectable_value(
                                            &mut self.settings.recording_fps,
                                            fps,
                                            format!("{} fps", fps),
                                        );
                                    }
                                });
                        });

                        ui.separator();

                        // Animation sequences
                        ui.horizontal(|ui| {
                            if ui.button("▶ Orbit 360°").clicked() {
                                self.show_left_side_panel = false;
                                self.animation = Some(animation::orbit_360(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                            if ui.button("▶ Axial Sweep").clicked() {
                                self.show_left_side_panel = false;
                                self.animation = Some(animation::axial_sweep(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("▶ Zoom Regions").clicked() {
                                self.show_left_side_panel = false;
                                self.animation =
                                    Some(animation::zoom_to_regions(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                            if ui.button("▶ Cinematic").clicked() {
                                self.show_left_side_panel = false;
                                self.animation = Some(animation::cinematic(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("▶ Hemisphere Split").clicked() {
                                self.show_left_side_panel = false;
                                self.animation =
                                    Some(animation::hemisphere_split(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                            if ui.button("▶ Top-Down Dive").clicked() {
                                self.show_left_side_panel = false;
                                self.animation =
                                    Some(animation::top_down_dive(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("▶ Pendulum").clicked() {
                                self.show_left_side_panel = false;
                                self.animation = Some(animation::pendulum(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                            if ui.button("▶ Spiral Zoom").clicked() {
                                self.show_left_side_panel = false;
                                self.animation = Some(animation::spiral_zoom(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("▶ Figure Eight").clicked() {
                                self.show_left_side_panel = false;
                                self.animation =
                                    Some(animation::figure_eight(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                            if ui.button("▶ Slow Reveal").clicked() {
                                self.show_left_side_panel = false;
                                self.animation = Some(animation::slow_reveal(self.camera.distance));
                                self.settings.recording = true;
                                self.settings.recording_delay = 5;
                            }
                        });

                        if self.animation.is_some() {
                            if ui.button("⏹ Stop Animation").clicked() {
                                self.show_left_side_panel = true;
                                self.animation = None;
                                self.settings.recording = false;
                            }
                        }
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

    fn resolve_adaptive_aa(&mut self) {
        use camera::MotionLevel;

        if self.settings.aa_mode == AntiAliasingMode::Adaptive {
            let target = match self.camera.motion_level() {
                MotionLevel::Fast => AntiAliasingMode::SMAA,
                MotionLevel::Slow => AntiAliasingMode::TAA,
                MotionLevel::Still => AntiAliasingMode::SSAA,
            };

            // update render_scale when the effective mode changes.
            if target != self.settings.effective_aa_mode {
                let new_scale = if target == AntiAliasingMode::SSAA {
                    2.0
                } else {
                    self.pre_ssaa_render_scale
                };
                if (self.settings.render_scale - new_scale).abs() > 0.001 {
                    self.settings.render_scale = new_scale;
                    self.settings.update_render_size();
                }
            }

            self.settings.effective_aa_mode = target;
        } else {
            self.settings.effective_aa_mode = self.settings.aa_mode;
        }
    }

    pub fn mouse_ndc(&self) -> glam::Vec2 {
        let mx = self.state.position.x / self.settings.width as f32 * 2.0 - 1.0;
        let my = -(self.state.position.y / self.settings.height as f32 * 2.0 - 1.0);
        glam::Vec2::new(mx.clamp(-1.0, 1.0), my.clamp(-1.0, 1.0))
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

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.settings.width = size.width;
        self.settings.height = size.height;
        self.settings.update_render_size();
    }

    pub fn time(&self) -> f32 {
        Instant::now().duration_since(self.time).as_secs_f32()
    }

    pub fn tick(&mut self, dt: f32) {
        self.animation_just_finished = false;

        if let Some(anim) = &mut self.animation {
            if let Some((yaw, pitch, distance)) = anim.update(dt) {
                self.camera.yaw = yaw;
                self.camera.pitch = pitch;
                self.camera.distance = distance;
            }

            if anim.finished() {
                self.animation_just_finished = true;
                self.animation = None;
            }
        }
    }
}

fn ternary_checkbox(ui: &mut Ui, input: Option<bool>, text: &str) -> Option<bool> {
    let mut checked = input.unwrap_or_default();

    ui.toggle_value(&mut checked, text)
        .clicked()
        .then_some(checked)
}
