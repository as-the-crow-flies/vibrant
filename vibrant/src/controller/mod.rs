pub mod camera;
pub mod components;
pub mod event;
pub mod icons;
pub mod light;
pub mod settings;
pub mod state;
pub mod widgets;

use camera::Camera;
use egui::{Align, CentralPanel, Frame, Layout, Ui};
use egui::{Panel, Rect};
use egui_dock::{DockArea, DockState, NodeIndex, Style as DockStyle};
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
use crate::{asset::Asset, file::FileStage, surface::accumulate::AccumulationStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum Tab {
    Slicing,
    #[default]
    Volumes,
    Masks,
    Tractography,
    Rendering,
    Controls,
}

impl Tab {
    fn icon_label(self) -> (&'static str, &'static str) {
        match self {
            Tab::Slicing => (icons::regular::CROP, "Slicing"),
            Tab::Volumes => (icons::regular::BRAIN, "Volumes"),
            Tab::Masks => (icons::regular::CIRCLE_HALF, "Masks"),
            Tab::Tractography => (icons::regular::PATH, "Tractography"),
            Tab::Rendering => (icons::regular::LIGHTBULB, "Rendering"),
            Tab::Controls => (icons::regular::MOUSE, "Controls"),
        }
    }
}

fn default_dock_state() -> DockState<Tab> {
    let mut dock_state = DockState::new(vec![Tab::Slicing, Tab::Rendering]);
    let surface = dock_state.main_surface_mut();

    let [_, data] = surface.split_below(
        NodeIndex::root(),
        0.4,
        vec![Tab::Volumes, Tab::Masks, Tab::Tractography],
    );

    let [_, rendering] = surface.split_below(data, 0.7, vec![Tab::Controls]);
    surface[rendering].set_collapsed(true);

    dock_state
}

/// Borrows the app state needed to render whichever tab is active, so
/// `egui_dock::DockArea` can dispatch to it without knowing about
/// `Controller`'s fields.
struct ControllerTabViewer<'a> {
    asset: &'a mut Asset,
    camera: &'a mut Camera,
    settings: &'a mut Settings,
    volumes_widget: &'a mut VolumesWidget,
    mask_widget: &'a mut MasksWidget,
    tractography_widget: &'a mut TractographyWidget,
    crop_widget: &'a mut CropWidget,
    rendering_widget: &'a mut RenderingWidget,
    controls_widget: &'a mut ControlsWidget,
    hdr_headroom_limit: Option<f32>,
    accumulation: AccumulationStatus,
}

impl egui_dock::TabViewer for ControllerTabViewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(*tab)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        let (icon, label) = tab.icon_label();
        format!("{icon} {label}").into()
    }

    fn is_closeable(&self, _tab: &Self::Tab) -> bool {
        false
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        match tab {
            // These widgets scroll their own item list, so they manage their
            // own `ScrollArea` instead of the dock body's.
            Tab::Volumes | Tab::Tractography => [false, false],
            _ => [false, true],
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Slicing => {
                self.crop_widget.show(ui, &mut self.asset.crop);
            }
            Tab::Volumes => {
                self.volumes_widget
                    .show(ui, &mut self.asset.volumes, &self.asset.masks);
            }
            Tab::Masks => {
                self.mask_widget.show(ui, &mut self.asset.masks);
            }
            Tab::Tractography => {
                self.tractography_widget
                    .show(ui, &mut self.asset.line, self.settings);
            }
            Tab::Rendering => {
                self.rendering_widget.show(
                    ui,
                    &mut self.asset.hdri,
                    self.asset.line.as_ref(),
                    self.camera,
                    self.settings,
                    self.hdr_headroom_limit,
                    self.accumulation,
                );
            }
            Tab::Controls => {
                self.controls_widget.show(ui);
            }
        }
    }
}

pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    settings: Settings,
    time: Instant,

    volumes_widget: VolumesWidget,
    mask_widget: MasksWidget,
    tractography_widget: TractographyWidget,
    crop_widget: CropWidget,
    rendering_widget: RenderingWidget,
    controls_widget: ControlsWidget,

    dock_state: DockState<Tab>,

    viewport: Rect,
    hovered: bool,
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::new(),
            settings: Settings::new(),
            time: Instant::now(),

            volumes_widget: VolumesWidget::new(),
            mask_widget: MasksWidget::new(),
            tractography_widget: TractographyWidget::new(),
            crop_widget: CropWidget::new(),
            rendering_widget: RenderingWidget::new(),
            controls_widget: ControlsWidget::new(),

            dock_state: default_dock_state(),

            viewport: Rect::ZERO,
            hovered: true,
        }
    }

    pub fn rendering(&self) -> &RenderingWidget {
        &self.rendering_widget
    }

    pub fn rendering_mut(&mut self) -> &mut RenderingWidget {
        &mut self.rendering_widget
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

    /// Bring the tab responsible for a data kind to the front. Called from
    /// `Asset::update` right after new files of that kind are loaded, so the
    /// dock tab system mirrors the old collapsing-header behaviour where
    /// loading a file auto-opened its section - tabs don't render at all
    /// while inactive, so that can no longer happen from inside the widget.
    pub fn focus_volumes_tab(&mut self) {
        self.focus_tab(Tab::Volumes);
    }

    pub fn focus_masks_tab(&mut self) {
        self.focus_tab(Tab::Masks);
    }

    pub fn focus_tractography_tab(&mut self) {
        self.focus_tab(Tab::Tractography);
    }

    fn focus_tab(&mut self, tab: Tab) {
        if let Some(path) = self.dock_state.find_tab(&tab) {
            let _ = self.dock_state.set_active_tab(path);
            self.dock_state[path.node_path()].set_collapsed(false);
            self.dock_state
                .set_focused_node_and_surface(path.node_path());
        }
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
        self.crop_widget.reset_changed();
        self.volumes_widget.reset_changed();
        self.mask_widget.reset_changed();
        self.tractography_widget.reset_changed();
        self.rendering_widget.reset_changed();

        Panel::top("TopBottomPanel").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("📂 open")
                    .on_hover_text(
                        "Load imaging data: NIfTI volumes (.nii / .nii.gz), \
                         tractograms (.tck), track scalars (.tsf), and .exr \
                         environment maps. A file whose name contains \"mask\" \
                         is loaded as a mask.",
                    )
                    .clicked()
                {
                    FileStage::load();
                }

                if ui
                    .button("📷 screenshot")
                    .on_hover_text(
                        "Save the current view as a PNG with a transparent \
                         background, ready to drop into a figure or slide.",
                    )
                    .clicked()
                {
                    FileStage::save();
                }

                if ui
                    .button(format!("{} GitHub", icons::regular::GITHUB_LOGO))
                    .on_hover_text("Open the VIBRANT source repository on GitHub.")
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(
                        "https://github.com/as-the-crow-flies/vibrant",
                    ));
                }

                ui.take_available_width();

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(format!("{:3.0} fps", 1.0 / dt));
                });
            });
        });

        Panel::left("SidePanelLeft")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .min_size(400.0)
            .max_size(1000.0)
            .show(ui, |ui| {
                ui.take_available_width();

                let mut viewer = ControllerTabViewer {
                    asset: &mut *asset,
                    camera: &mut self.camera,
                    settings: &mut self.settings,
                    volumes_widget: &mut self.volumes_widget,
                    mask_widget: &mut self.mask_widget,
                    tractography_widget: &mut self.tractography_widget,
                    crop_widget: &mut self.crop_widget,
                    rendering_widget: &mut self.rendering_widget,
                    controls_widget: &mut self.controls_widget,
                    hdr_headroom_limit,
                    accumulation,
                };

                DockArea::new(&mut self.dock_state)
                    .style(DockStyle::from_egui(ui.style()))
                    .show_leaf_close_all_buttons(false)
                    .show_close_buttons(false)
                    .show_inside(ui, &mut viewer);
            });

        let viewport = CentralPanel::no_frame().show(ui, |ui| {
            if asset.volumes.is_empty() && asset.line.is_none() && !FileStage::loading() {
                ui.painter().text(
                    ui.max_rect().center(),
                    egui::Align2::CENTER_CENTER,
                    "Open your first .nii.gz or .tck file using the [📂 open] button",
                    egui::FontId::proportional(20.0),
                    ui.visuals().weak_text_color(),
                );
            }

            if FileStage::loading() {
                let rect =
                    egui::Rect::from_center_size(ui.max_rect().center(), egui::Vec2::splat(48.0));
                egui::Spinner::new().size(32.0).paint_at(ui, rect);
            }
        });

        self.hovered = viewport.response.hovered();
        self.viewport = viewport.response.rect * scale;
        self.camera.set_aspect(self.viewport.aspect_ratio());
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

    /// Set the render viewport directly, for headless callers (benches) that
    /// never run the egui pass that normally derives it from the central panel.
    /// Mirrors the `self.viewport` / `set_aspect` lines in [`Self::ui`].
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width, height));
        self.camera.set_aspect(self.viewport.aspect_ratio());
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
