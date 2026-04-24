use std::any::type_name;

use egui::{collapsing_header::CollapsingState, ComboBox, Grid, Ui};
use egui_double_slider::DoubleSlider;
use strum::IntoEnumIterator;

use crate::{
    asset::{
        volume_fraction::{MaterialPreset, VolumeFractionBuffer},
        volume_mask::VolumeMaskBuffer,
    },
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct VolumesWidget {
    changed: bool,
}

impl Tracked for VolumesWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl VolumesWidget {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        volumes: &mut Vec<VolumeFractionBuffer>,
        masks: &[VolumeMaskBuffer],
    ) {
        self.changed = false;

        if volumes.is_empty() {
            return;
        }

        let mut index_to_remove: Option<usize> = None;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), true)
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
                        ui.checkbox(&mut settings.visible, "").track(self);
                        ui.text_edit_singleline(&mut settings.name).track(self);
                        ui.toggle_value(&mut settings.inverted, "🌗").track(self);

                        if ui.button("🗑").clicked() {
                            index_to_remove = Some(index);
                            self.track();
                        }
                    })
                    .body(|ui| {
                        Grid::new("VolumeSettingsGrid").show(ui, |ui| {
                            ui.label("Contrast");
                            ui.add(
                                DoubleSlider::new(&mut settings.min, &mut settings.max, 0.0..=1.0)
                                    .width(300.0)
                                    .separation_distance(0.01),
                            )
                            .track(self);

                            ui.end_row();

                            ui.label("Mask");
                            ui.horizontal(|ui| {
                                ComboBox::from_id_salt("VolumeMask")
                                    .selected_text(masks[settings.mask].settings().name.clone())
                                    .show_ui(ui, |ui| {
                                        for (index, mask) in masks.iter().enumerate() {
                                            ui.selectable_value(
                                                &mut settings.mask,
                                                index,
                                                mask.settings().name.clone(),
                                            )
                                            .track(self);
                                        }
                                    });
                            });

                            ui.end_row();

                            ui.label("Material");

                            ui.horizontal(|ui| {
                                let mut preset_changed = false;

                                ComboBox::from_id_salt("MaterialPreset")
                                    .selected_text(format!("{:?}", settings.preset))
                                    .show_ui(ui, |ui| {
                                        for value in MaterialPreset::iter() {
                                            let label = format!("{:?}", value);
                                            preset_changed |= ui
                                                .selectable_value(
                                                    &mut settings.preset,
                                                    value,
                                                    label,
                                                )
                                                .track(self);
                                        }
                                    });

                                if preset_changed {
                                    (settings.absorption, settings.scattering) =
                                        settings.preset.into();
                                }

                                let mut material_changed = false;

                                material_changed |= ui
                                    .color_edit_button_rgb(&mut settings.absorption)
                                    .track(self);
                                material_changed |= ui
                                    .color_edit_button_rgb(&mut settings.scattering)
                                    .track(self);

                                if material_changed {
                                    settings.preset = MaterialPreset::Custom;
                                }
                            });

                            ui.end_row();
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
