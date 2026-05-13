use std::iter::zip;

use egui::{ComboBox, Grid, RichText, Ui};
use egui_double_slider::DoubleSlider;
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    asset::{
        colormap::ColormapSelection,
        volume_fraction::{MaterialPreset, VolumeFractionBuffer, VolumeFractionSettings},
        volume_mask::VolumeMaskBuffer,
    },
    controller::{components::UIComponents, widgets::util::UiResponseExtensions},
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

        let mut index_to_remove: Option<usize> = None;

        let open = volumes
            .iter()
            .map(|volume| ui.is_new(&volume.settings().name))
            .collect_vec();

        ui.collapse(
            RichText::new("Volumes").heading(),
            open.iter().any(|&x| x),
            |ui| {
                for (index, (volume, open)) in zip(volumes.iter_mut(), open).enumerate() {
                    let volume = volume.settings_mut();

                    ui.frame(|ui| {
                        let title = volume.name.clone();
                        let response = ui.collapse(title, open, |ui| {
                            self.show_volume(ui, volume, masks);
                        });

                        ui.inline(&response, |ui| {
                            if ui.delete().track(self).clicked() {
                                index_to_remove = Some(index);
                            }
                            ui.toggle_inverted(&mut volume.inverted).track(self);
                            ui.toggle_visible(&mut volume.visible).track(self);
                        });
                    });
                }
            },
        )
        .help(
            "NIfTI Volume Rendering",
            "Open .nii.gz files to render them volumetrically.",
        );

        if let Some(index) = index_to_remove {
            volumes.remove(index);
            self.track();
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    fn show_volume(
        &mut self,
        ui: &mut Ui,
        volume: &mut VolumeFractionSettings,
        masks: &[VolumeMaskBuffer],
    ) {
        Grid::new("VolumeSettingsGrid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Mask").on_hover_text(
                    "Mask from the 'Masks' section to apply to this volume.",
                );
                ui.horizontal(|ui| {
                    ComboBox::from_id_salt("VolumeMask")
                        .selected_text(
                            masks[volume.mask].settings().name.clone(),
                        )
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for (index, mask) in masks.iter().enumerate() {
                                ui.selectable_value(
                                    &mut volume.mask,
                                    index,
                                    mask.settings().name.clone(),
                                )
                                .track(self);
                            }
                        }).response.on_hover_text(
                            "Mask from the 'Masks' section to apply to this volume.\nChoose 'None' to disable masking.",
                        );
                });
                ui.end_row();

                ui.label("Contrast").on_hover_text("Adjust mapping from volume min/max to display min/max values");
                ui.add(
                    DoubleSlider::new(
                        &mut volume.min,
                        &mut volume.max,
                        0.0..=1.0,
                    )
                    .width(ui.available_width())
                    .separation_distance(0.01),
                )
                .track(self);

                ui.end_row();

                ui.label("Opacity").on_hover_text("Adjust volume opacity");
                ui.slider(&mut volume.opacity, 0.0..=2.0).track(self);
                ui.end_row();

                ui.label("Material").on_hover_text("Adjust volume appearance");
                ui.horizontal(|ui| {
                    let mut material_changed = false;

                    material_changed |= ui
                        .color_edit_button_rgb(&mut volume.absorption)
                        .on_hover_text("Volume Absorption.\nHow much light is absorbed by the volume.")
                        .track(self)
                        .changed();

                    material_changed |= ui
                        .color_edit_button_rgb(&mut volume.scattering)
                        .on_hover_text("Volume Scattering.\nHow much light is scattered by the volume.")
                        .track(self)
                        .changed();

                    if material_changed {
                        volume.preset = MaterialPreset::Custom;
                    }

                    let mut preset_changed = false;

                    ComboBox::from_id_salt("MaterialPreset")
                        .selected_text(format!("{:?}", volume.preset))
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for value in MaterialPreset::iter() {
                                let label = format!("{:?}", value);
                                preset_changed |= ui
                                    .selectable_value(
                                        &mut volume.preset,
                                        value,
                                        label,
                                    )
                                    .track(self)
                                    .changed();
                            }
                        }).response.on_hover_text("Material Preset");

                    if preset_changed {
                        (volume.absorption, volume.scattering) =
                            volume.preset.into();
                    }
                });

                ui.end_row();

                ui.label("Colormap").on_hover_text("Colormap to apply to volume. The colormap is applied after contrast and material settings are applied.");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut volume.use_colormap, "").on_hover_text("Enable/Disable Colormap").track(self);
                    ComboBox::from_id_salt(format!(
                        "{}_VolumeColormap",
                        volume.name
                    ))
                    .width(ui.available_width())
                    .selected_text(format!("{:?}", volume.colormap))
                    .show_ui(
                        ui,
                        |ui| {
                            for map in ColormapSelection::iter() {
                                ui.selectable_value(
                                    &mut volume.colormap,
                                    map,
                                    format!("{:?}", map),
                                )
                                .track(self);
                            }
                        },
                    ).response.on_hover_text("Colormap Preset");
                })
            });
    }
}
