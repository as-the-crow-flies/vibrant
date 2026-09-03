use std::iter::zip;

use egui::{ComboBox, Grid, RichText, Ui};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    asset::{
        colormap::ColormapSelection,
        volume_fraction::{VolumeFractionBuffer, VolumeFractionSettings},
        volume_mask::VolumeMaskBuffer,
    },
    controller::{
        components::{transfer::TransferFunctionEditor, UIComponents},
        icons,
        widgets::util::UiResponseExtensions,
    },
    util::{ResponseExtentions, Tracked},
};

pub struct VolumesWidget {
    transfer: TransferFunctionEditor,
    changed: bool,
}

impl Tracked for VolumesWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl VolumesWidget {
    pub fn new() -> Self {
        Self {
            transfer: TransferFunctionEditor::new(),
            changed: false,
        }
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
            RichText::new(format!("{} Volumes", icons::regular::BRAIN)).heading(),
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
        self.transfer.show(ui, volume);
        self.changed |= self.transfer.changed();

        ui.separator();

        Grid::new("VolumeSettingsGrid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Opacity").on_hover_text(
                    "Scales this volume's absorption and scattering, after the transfer function is applied.",
                );
                ui.slider(&mut volume.opacity, 0.0..=1.0).track(self);
                ui.end_row();

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
