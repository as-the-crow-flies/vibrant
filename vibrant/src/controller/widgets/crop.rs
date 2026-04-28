use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Grid, Ui};
use egui_double_slider::DoubleSlider;

use crate::{
    controller::settings::Settings,
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

    pub fn show(&mut self, ui: &mut Ui, settings: &mut Settings) {
        self.changed = false;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), true)
            .show_header(ui, |ui| ui.heading("Crop"))
            .body(|ui| {
                Grid::new("CropWidgetGrid").num_columns(2).show(ui, |ui| {
                    self.slider(
                        ui,
                        "Axial",
                        &mut settings.crop_z_start,
                        &mut settings.crop_z_end,
                    );

                    self.slider(
                        ui,
                        "Sagittal",
                        &mut settings.crop_x_start,
                        &mut settings.crop_x_end,
                    );

                    self.slider(
                        ui,
                        "Coronal",
                        &mut settings.crop_y_start,
                        &mut settings.crop_y_end,
                    );
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
