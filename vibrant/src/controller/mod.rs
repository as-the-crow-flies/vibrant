pub mod camera;
pub mod components;
pub mod event;
pub mod icons;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;
pub mod widgets;

use camera::Camera;
use egui::{Align, CentralPanel, Layout, ScrollArea, Ui};
use egui::{Panel, Rect};
use event::Event;
use light::Light;
use settings::Settings;
use state::ControllerState;
use web_time::Instant;
use winit::dpi::PhysicalSize;

use crate::controller::widgets::controls::ControlsWidget;
use crate::controller::widgets::{
    crop::CropWidget, masks::MasksWidget, rendering::RenderingWidget,
    tractography::TractographyWidget, volumes::VolumesWidget,
};
use crate::{
    asset::Asset, controller::segment::Segment, file::FileStage,
    surface::accumulate::AccumulationStatus,
};

pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    segment: Segment,
    settings: Settings,
    time: Instant,

    volumes_widget: VolumesWidget,
    mask_widget: MasksWidget,
    tractography_widget: TractographyWidget,
    crop_widget: CropWidget,
    rendering_widget: RenderingWidget,
    controls_widget: ControlsWidget,

    viewport: Rect,
    hovered: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::new(),
            segment: Segment::new(),
            settings: Settings::new(),
            time: Instant::now(),

            volumes_widget: VolumesWidget::new(),
            mask_widget: MasksWidget::new(),
            tractography_widget: TractographyWidget::new(),
            crop_widget: CropWidget::new(),
            rendering_widget: RenderingWidget::new(),
            controls_widget: ControlsWidget::new(),

            viewport: Rect::ZERO,
            hovered: true,
        }
    }

    pub fn rendering(&self) -> &RenderingWidget {
        &self.rendering_widget
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

    pub fn radiance(&self) -> &RenderingWidget {
        &self.rendering_widget
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);

        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        asset: &mut Asset,
        scale: f32,
        dt: f32,
        hdr_headroom_limit: Option<f32>,
        // Live accumulation progress from the renderer.
        accumulation: AccumulationStatus,
    ) {
        Panel::top("TopBottomPanel").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("📷 screenshot")
                    .on_hover_text("Take screenshot with transparent background")
                    .clicked()
                {
                    FileStage::save();
                }

                if ui
                    .button(format!("{} GitHub", icons::regular::GITHUB_LOGO))
                    .on_hover_text("View on GitHub")
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(
                        "https://github.com/as-the-crow-flies/vibrant",
                    ));
                }

                ui.take_available_width();

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button("📂 open")
                        .on_hover_text("Open .nii.gz/.tck/.tsf files")
                        .clicked()
                    {
                        FileStage::load();
                    }

                    ui.label(format!("{:3.0} fps", 1.0 / dt));
                });
            });
        });

        Panel::right("SidePanelRight")
            .min_size(400.0)
            .max_size(800.0)
            .show(ui, |ui| {
                ui.take_available_width();

                ScrollArea::new([false, true]).show(ui, |ui| {
                    self.crop_widget.show(ui, &mut asset.crop);

                    self.volumes_widget
                        .show(ui, &mut asset.volumes, &mut asset.masks);

                    self.mask_widget.show(ui, &mut asset.masks);

                    self.tractography_widget
                        .show(ui, &mut asset.line, &mut self.settings);

                    self.rendering_widget.show(
                        ui,
                        &mut asset.hdri,
                        &mut self.camera,
                        &mut self.settings,
                        hdr_headroom_limit,
                        accumulation,
                    );

                    self.controls_widget.show(ui);
                });
            });

        let viewport = CentralPanel::no_frame().show(ui, |ui| {
            if asset.volumes.is_empty() && asset.line.is_none() {
                ui.painter().text(
                    ui.max_rect().center(),
                    egui::Align2::CENTER_CENTER,
                    "Open your first .nii.gz or .tck file using the [📂 open] button",
                    egui::FontId::proportional(20.0),
                    ui.visuals().weak_text_color(),
                );
            }
        });

        self.hovered = viewport.response.hovered();
        self.viewport = viewport.response.rect * scale;
        self.camera.aspect = self.viewport.aspect_ratio();
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Whether the view moved since the last call (rotate/pan/zoom/reset),
    /// clearing the flag. Used by the renderer to reset frame accumulation.
    pub fn take_camera_changed(&mut self) -> bool {
        self.camera.take_changed()
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

    pub fn lighting_changed(&self) -> bool {
        self.changed() || self.light().changed()
    }

    pub fn changed(&self) -> bool {
        self.crop().changed()
            || self.volumes().changed()
            || self.masks().changed()
            || self.tractography().changed()
            || self.rendering().changed()
    }
}
