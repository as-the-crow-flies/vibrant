pub mod camera;
pub mod event;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;
pub mod widgets;

use camera::Camera;
use egui::ComboBox;
use egui::{Align, Frame, Layout, Margin, ScrollArea, SidePanel, Slider, Ui};
use event::Event;
use light::Light;
use settings::Settings;
use state::ControllerState;
use web_time::Instant;
use winit::dpi::PhysicalSize;

use crate::controller::widgets::crop::CropWidget;
use crate::controller::widgets::hdri::HdriWidget;
use crate::controller::widgets::tractography::TractographyWidget;
use crate::controller::widgets::volumes::VolumesWidget;
use crate::{asset::Asset, controller::segment::Segment, file::FileStage};

#[derive(Debug)]
pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    segment: Segment,
    settings: Settings,
    time: Instant,

    volumes_widget: VolumesWidget,
    tractography_widget: TractographyWidget,
    crop_widget: CropWidget,
    hdri_widget: HdriWidget,

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
            time: Instant::now(),

            volumes_widget: VolumesWidget::new(),
            tractography_widget: TractographyWidget::new(),
            crop_widget: CropWidget::new(),
            hdri_widget: HdriWidget::new(),

            show_left_side_panel: false,
            show_right_side_panel: true,
        }
    }

    pub fn hdri(&self) -> &HdriWidget {
        &self.hdri_widget
    }

    pub fn crop(&self) -> &CropWidget {
        &self.crop_widget
    }

    pub fn tractography(&self) -> &TractographyWidget {
        &self.tractography_widget
    }

    pub fn volumes(&self) -> &VolumesWidget {
        &self.volumes_widget
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
                            Slider::new(&mut self.settings.lighting, 0.0..=10.0).text("Lighting"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.ambient_light, 0.0..=10.0)
                                .text("Ambient Light"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.direct_light, 0.0..=1.0)
                                .text("Direct Light"),
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

                        ui.add(Slider::new(&mut self.settings.plane, 0.0..=1.0).text("plane"));

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
                ScrollArea::new([false, true]).show(ui, |ui| {
                    self.crop_widget.show(ui, &mut self.settings);

                    self.volumes_widget.show(ui, &mut asset.volumes);

                    if let Some(lines) = &mut asset.line {
                        self.tractography_widget.show(ui, lines);
                    }

                    if let Some(hdri) = &mut asset.hdri {
                        self.hdri_widget.show(ui, hdri);
                    }
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
