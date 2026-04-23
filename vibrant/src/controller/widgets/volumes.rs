use std::{
    any::type_name,
    hash::{DefaultHasher, Hash, Hasher},
};

use egui::{collapsing_header::CollapsingState, ComboBox, Ui};
use egui_double_slider::DoubleSlider;
use strum::IntoEnumIterator;

use crate::asset::volume_fraction::{MaterialPreset, VolumeFractionBuffer};

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
                        true,
                    )
                    .show_header(ui, |ui| {
                        ui.text_edit_singleline(&mut settings.name);

                        self.changed |= ui.toggle_value(&mut settings.visible, "👁").changed();
                        self.changed |= ui.toggle_value(&mut settings.inverted, "🌗").changed();
                        self.changed |= ui.toggle_value(&mut settings.masked, "▣").changed();

                        if ui.button("🗑").clicked() {
                            index_to_remove = Some(index);
                            self.changed = true;
                        }
                    })
                    .body(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Material");

                            let mut material_changed = false;

                            material_changed |=
                                ui.color_edit_button_rgb(&mut settings.absorption).changed();
                            material_changed |=
                                ui.color_edit_button_rgb(&mut settings.scattering).changed();

                            let mut preset_changed = false;

                            ComboBox::from_id_salt("MaterialPreset")
                                .selected_text(format!("{:?}", settings.preset))
                                .show_ui(ui, |ui| {
                                    for value in MaterialPreset::iter() {
                                        let label = format!("{:?}", value);
                                        preset_changed |= ui
                                            .selectable_value(&mut settings.preset, value, label)
                                            .changed();
                                    }
                                });

                            if material_changed {
                                settings.preset = MaterialPreset::Custom;
                            }

                            if preset_changed {
                                (settings.absorption, settings.scattering) = settings.preset.into();
                            }

                            self.changed |= material_changed || preset_changed;
                        });

                        ui.horizontal(|ui| {
                            ui.label("Contrast");
                            self.changed |= ui
                                .add(
                                    DoubleSlider::new(
                                        &mut settings.min,
                                        &mut settings.max,
                                        0.0..=1.0,
                                    )
                                    .width(300.0)
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
