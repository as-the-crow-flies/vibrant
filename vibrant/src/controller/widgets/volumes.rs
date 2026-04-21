use std::{
    any::type_name,
    hash::{DefaultHasher, Hash, Hasher},
};

use egui::{collapsing_header::CollapsingState, Ui};
use egui_double_slider::DoubleSlider;

use crate::asset::volume_fraction::VolumeFractionBuffer;

#[derive(Debug)]
pub struct VolumesWidget {
    hash: u64,
    changed: bool,
}

impl VolumesWidget {
    pub fn new() -> Self {
        Self {
            changed: false,
            hash: 0,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, volumes: &mut Vec<VolumeFractionBuffer>) {
        if volumes.is_empty() {
            return;
        }

        self.changed = false;

        let mut hasher = DefaultHasher::new();
        for volume in volumes.iter() {
            volume.settings().name.hash(&mut hasher);
        }
        let hash = hasher.finish();

        self.changed = hash != self.hash;
        self.hash = hash;

        let mut index_to_remove: Option<usize> = None;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), self.changed)
            .show_header(ui, |ui| ui.heading("Volumes"))
            .body(|ui| {
                for (index, volume) in volumes.iter_mut().enumerate() {
                    let settings = volume.settings_mut();

                    CollapsingState::load_with_default_open(
                        ui.ctx(),
                        settings.name.to_string().into(),
                        false,
                    )
                    .show_header(ui, |ui| {
                        if ui.button("🗑").clicked() {
                            index_to_remove = Some(index);
                            self.changed = true;
                        }

                        self.changed |= ui.checkbox(&mut settings.visible, "").changed();

                        self.changed |=
                            ui.color_edit_button_rgb(&mut settings.absorption).changed();

                        self.changed |=
                            ui.color_edit_button_rgb(&mut settings.scattering).changed();

                        ui.text_edit_singleline(&mut settings.name);
                    })
                    .body(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Contrast");
                            self.changed |= ui
                                .add(
                                    DoubleSlider::new(
                                        &mut settings.user_min,
                                        &mut settings.user_max,
                                        0.0..=1.0,
                                    )
                                    .width(400.0)
                                    .separation_distance(0.01),
                                )
                                .changed();
                        });
                    });
                }
            });

        if let Some(index) = index_to_remove {
            volumes.remove(index);
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
