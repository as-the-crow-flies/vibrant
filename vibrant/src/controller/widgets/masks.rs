use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Slider, Ui};

use crate::{
    asset::volume_mask::VolumeMaskBuffer,
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct MasksWidget {
    changed: bool,
}

impl Tracked for MasksWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl MasksWidget {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn show(&mut self, ui: &mut Ui, masks: &mut Vec<VolumeMaskBuffer>) {
        self.changed = false;

        if masks.len() <= 1 {
            return;
        }

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), true)
            .show_header(ui, |ui| {
                ui.heading("Masks");
            })
            .body(|ui| {
                ui.spacing_mut().slider_width = 300.0;

                for mask in masks.iter_mut() {
                    let settings = mask.settings_mut();

                    // Don't show default mask
                    if settings.name == "None" {
                        continue;
                    }

                    CollapsingState::load_with_default_open(
                        ui.ctx(),
                        settings.name.to_string().into(),
                        true,
                    )
                    .show_header(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut settings.visible, "").track(self);
                            ui.text_edit_singleline(&mut settings.name).track(self);
                            ui.toggle_value(&mut settings.inverted, "🌗").track(self);
                        });
                    })
                    .body(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Offset");
                            ui.add(Slider::new(&mut settings.offset, -0.5..=0.5))
                                .track(self);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Width");
                            ui.add(Slider::new(&mut settings.width, 0.01..=0.1))
                                .track(self);
                        });
                    });
                }
            });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
