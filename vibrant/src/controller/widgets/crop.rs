use std::f32::consts::PI;

use egui::{Grid, RichText, Ui};
use egui_double_slider::DoubleSlider;

use crate::{
    asset::crop::CropBuffer,
    controller::{components::UIComponents, widgets::util::UiResponseExtensions},
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

        ui.collapse(RichText::new("Slicing").heading(), false, |ui| {
            ui.collapse("Orthogonal", false, |ui| {
                Grid::new("CropWidgetGridOrthogonal")
                    .num_columns(2)
                    .show(ui, |ui| {
                        self.slider(ui, "Axial", &mut s.min.z, &mut s.max.z);
                        self.slider(ui, "Sagittal", &mut s.min.x, &mut s.max.x);
                        self.slider(ui, "Coronal", &mut s.min.y, &mut s.max.y);
                    });
            });

            ui.collapse("Spherical", false, |ui| {
                Grid::new("CropWidgetGridSpherical")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Azimuth");
                        ui.slider(&mut s.spherical.x, 0.0..=2.0 * PI).track(self);
                        ui.end_row();

                        ui.label("Elevation");
                        ui.slider(&mut s.spherical.y, 0.0..=PI).track(self);
                        ui.end_row();

                        ui.label("Depth");
                        ui.slider(&mut s.spherical.z, 0.0..=1.0).track(self);
                        ui.end_row();

                        ui.label("Smooth");
                        ui.slider(&mut s.spherical.w, 0.0..=1.0).track(self);
                        ui.end_row();
                    });
            });
        })
        .help(
            "Slicing",
            "Crop volume using orthogonal and spherical slice controls.",
        );
    }

    fn slider(&mut self, ui: &mut Ui, label: &str, min: &mut f32, max: &mut f32) {
        ui.label(label);

        ui.add(
            DoubleSlider::new(min, max, -0.5..=0.5)
                .width(ui.available_width())
                .separation_distance(0.001),
        )
        .track(self);

        ui.end_row();
    }
}
