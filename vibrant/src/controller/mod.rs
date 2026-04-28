pub mod camera;
pub mod components;
pub mod event;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;
pub mod widgets;

use camera::Camera;
use egui::{Align, CentralPanel, Frame, Layout, Margin, ScrollArea, Ui};
use egui::{Panel, Rect};
use event::Event;
use light::Light;
use settings::Settings;
use state::ControllerState;
use web_time::Instant;
use winit::dpi::PhysicalSize;

use crate::controller::widgets::crop::CropWidget;
use crate::controller::widgets::hdri::HdriWidget;
use crate::controller::widgets::masks::MasksWidget;
use crate::controller::widgets::settings::SettingsWidget;
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

    settings_widget: SettingsWidget,
    volumes_widget: VolumesWidget,
    mask_widget: MasksWidget,
    tractography_widget: TractographyWidget,
    crop_widget: CropWidget,
    hdri_widget: HdriWidget,

    show_left_side_panel: bool,
    show_right_side_panel: bool,

    viewport: Rect,
    hovered: bool,
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

            settings_widget: SettingsWidget::new(),
            volumes_widget: VolumesWidget::new(),
            mask_widget: MasksWidget::new(),
            tractography_widget: TractographyWidget::new(),
            crop_widget: CropWidget::new(),
            hdri_widget: HdriWidget::new(),

            show_left_side_panel: false,
            show_right_side_panel: true,

            viewport: Rect::ZERO,
            hovered: true,
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

    pub fn masks(&self) -> &MasksWidget {
        &self.mask_widget
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);

        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(&mut self, ui: &mut Ui, asset: &mut Asset, scale: f32, _dt: f32) {
        Panel::top("TopBottomPanel").show_inside(ui, |ui| {
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

        Panel::left("SidePanelLeft").show_animated_inside(ui, self.show_left_side_panel, |ui| {
            Panel::top("top_panel")
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
                    self.settings_widget
                        .show(ui, &mut self.settings, &mut self.camera);
                });

            Panel::bottom("bottom_panel")
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

        Panel::right("SidePanelRight")
            .min_size(400.0)
            .max_size(800.0)
            .show_animated_inside(ui, self.show_right_side_panel, |ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    self.crop_widget.show(ui, &mut self.settings);

                    self.volumes_widget
                        .show(ui, &mut asset.volumes, &mut asset.masks);

                    self.mask_widget.show(ui, &mut asset.masks);

                    if let Some(lines) = &mut asset.line {
                        self.tractography_widget.show(ui, lines);
                    }

                    self.hdri_widget.show(ui, &mut asset.hdri);
                });
            });

        let viewport = CentralPanel::no_frame().show_inside(ui, |_| {});

        self.hovered = viewport.response.hovered();
        self.viewport = viewport.response.rect * scale;
        self.camera.aspect = self.viewport.aspect_ratio();
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

    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    pub fn hovered(&self) -> bool {
        self.hovered
    }

    pub fn changed(&self) -> bool {
        self.settings_widget.changed()
            | self.crop().changed()
            | self.volumes().changed()
            | self.masks().changed()
            | self.tractography().changed()
            | self.hdri().changed()
    }
}
