use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Align, ComboBox, Grid, Layout, RichText, Ui};
use egui_double_slider::DoubleSlider;
use strum::IntoEnumIterator;

use crate::{
    asset::{
        colormap::ColormapSelection,
        volume_fraction::{MaterialPreset, VolumeFractionBuffer},
        volume_mask::VolumeMaskBuffer,
    },
    controller::components::UIComponents,
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
                    let volume = volume.settings_mut();

                    ui.frame(|ui| {
                        CollapsingState::load_with_default_open(
                            ui.ctx(),
                            volume.name.to_string().into(),
                            true,
                        )
                        .show_header(ui, |ui| {
                            ui.label(RichText::new(&volume.name).strong());

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("🗑").clicked() {
                                    index_to_remove = Some(index);
                                    self.track();
                                }
                                ui.toggle_inverted(&mut volume.inverted).track(self);
                                ui.toggle_visible(&mut volume.visible).track(self);
                            });
                        })
                        .body(|ui| {
                            Grid::new("VolumeSettingsGrid")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label("Mask");
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
                                            });
                                    });
                                    ui.end_row();

                                    ui.label("Contrast");
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

                                    ui.label("Opacity");
                                    ui.slider(&mut volume.opacity, 0.0..=1.0).track(self);
                                    ui.end_row();

                                    ui.label("Material");

                                    ui.horizontal(|ui| {
                                        let mut material_changed = false;

                                        material_changed |= ui
                                            .color_edit_button_rgb(&mut volume.absorption)
                                            .track(self);
                                        material_changed |= ui
                                            .color_edit_button_rgb(&mut volume.scattering)
                                            .track(self);

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
                                                        .track(self);
                                                }
                                            });

                                        if preset_changed {
                                            (volume.absorption, volume.scattering) =
                                                volume.preset.into();
                                        }
                                    });

                                    ui.end_row();

                                    ui.label("Colormap");
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut volume.use_colormap, "").track(self);
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
                                        );
                                    })
                                });
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
