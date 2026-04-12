use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Slider, Ui};

use crate::controller::settings::Settings;

#[derive(Debug)]
pub struct CropWidget {
    changed: bool,
}

impl CropWidget {
    pub fn new() -> Self {
        Self { changed: true }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn show(&mut self, ui: &mut Ui, settings: &mut Settings) {
        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), self.changed)
            .show_header(ui, |ui| ui.heading("Crop"))
            .body(|ui| {
                self.changed = Self::sliders(
                    ui,
                    "X",
                    &mut settings.crop_x_start,
                    &mut settings.crop_x_end,
                ) | Self::sliders(
                    ui,
                    "Y",
                    &mut settings.crop_y_start,
                    &mut settings.crop_y_end,
                ) | Self::sliders(
                    ui,
                    "Z",
                    &mut settings.crop_z_start,
                    &mut settings.crop_z_end,
                );
            });
    }

    fn sliders(ui: &mut Ui, label: &str, min: &mut f32, max: &mut f32) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label(label);
            changed = Self::slider(ui, min) | Self::slider(ui, max);
        });

        changed
    }

    fn slider(ui: &mut Ui, value: &mut f32) -> bool {
        ui.add(Slider::new(value, -0.5..=0.5)).changed()
    }
}
