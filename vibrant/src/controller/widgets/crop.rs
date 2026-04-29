use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Grid, Ui};
use egui_double_slider::DoubleSlider;

use crate::{
    asset::crop::CropBuffer,
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

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), true)
            .show_header(ui, |ui| ui.heading("Crop"))
            .body(|ui| {
                Grid::new("CropWidgetGrid").num_columns(2).show(ui, |ui| {
                    self.slider(ui, "Axial", &mut s.min.z, &mut s.max.z);
                    self.slider(ui, "Sagittal", &mut s.min.x, &mut s.max.x);
                    self.slider(ui, "Coronal", &mut s.min.y, &mut s.max.y);
                });
            });
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
