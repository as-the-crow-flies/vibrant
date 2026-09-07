use std::f32::consts::PI;

use egui::{Grid, Ui};
use egui_double_slider::DoubleSlider;

use crate::{
    asset::crop::CropBuffer,
    controller::{components::UIComponents, icons, widgets::util::UiResponseExtensions},
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct CropWidget {
    changed: bool,
}

impl Tracked for CropWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl Default for CropWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl CropWidget {
    pub fn new() -> Self {
        Self { changed: true }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn show(&mut self, ui: &mut Ui, crop: &mut CropBuffer) {
        self.changed = false;

        let s = crop.settings_mut();

        let orthogonal_open = ui.is_new("Slicing.Orthogonal");
        ui.collapse(
            format!("{} Orthogonal", icons::regular::GRID_FOUR),
            orthogonal_open,
            |ui| {
                Grid::new("CropWidgetGridOrthogonal")
                    .num_columns(2)
                    .show(ui, |ui| {
                        self.slider(ui, "Axial", &mut s.min.z, &mut s.max.z);
                        self.slider(ui, "Sagittal", &mut s.min.x, &mut s.max.x);
                        self.slider(ui, "Coronal", &mut s.min.y, &mut s.max.y);
                    });
            },
        )
        .help(
            "Slicing",
            "Cut away parts of the scene to see inside. Orthogonal planes clip \
             along the anatomical axes; the spherical control carves out a \
             rounded window. Drag the two handles on each axis to keep only the \
             slab between them.",
        );

        let spherical_open = ui.is_new("Slicing.Spherical");
        ui.collapse(
            format!("{} Spherical", icons::regular::SPHERE),
            spherical_open,
            |ui| {
                Grid::new("CropWidgetGridSpherical")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Azimuth").on_hover_text(
                            "Direction the spherical cut faces, rotating around \
                             the vertical axis.",
                        );
                        ui.slider(&mut s.spherical.x, 0.0..=2.0 * PI).track(self);
                        ui.end_row();

                        ui.label("Elevation").on_hover_text(
                            "Tilt of the spherical cut, from below the scene to \
                             above it.",
                        );
                        ui.slider(&mut s.spherical.y, 0.0..=PI).track(self);
                        ui.end_row();

                        ui.label("Depth").on_hover_text(
                            "How far the spherical cut reaches into the scene.",
                        );
                        ui.slider(&mut s.spherical.z, 0.0..=1.0).track(self);
                        ui.end_row();

                        ui.label("Smooth").on_hover_text(
                            "Softness of the spherical cut's edge, from a hard \
                             boundary to a wide fade.",
                        );
                        ui.slider(&mut s.spherical.w, 0.0..=1.0).track(self);
                        ui.end_row();
                    });
            },
        );
    }

    fn slider(&mut self, ui: &mut Ui, label: &str, min: &mut f32, max: &mut f32) {
        ui.label(label).on_hover_text(format!(
            "Clip along the {} axis. Drag the two handles to keep only the slab \
             between them.",
            label.to_lowercase(),
        ));

        ui.add(
            DoubleSlider::new(min, max, -0.5..=0.5)
                .control_point_radius(5.0)
                .width(ui.available_width())
                .separation_distance(0.001),
        )
        .track(self);

        ui.end_row();
    }
}
