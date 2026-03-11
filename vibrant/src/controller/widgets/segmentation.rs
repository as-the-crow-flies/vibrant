use std::{
    any::type_name,
    hash::{DefaultHasher, Hash, Hasher},
};

use egui::{collapsing_header::CollapsingState, ScrollArea, Ui};

use crate::asset::segmentation::VolumeSegmenationBuffer;

#[derive(Debug)]
pub struct SegmentationsWidget {
    hash: u64,
    changed: bool,
}

impl SegmentationsWidget {
    pub fn new() -> Self {
        Self {
            changed: false,
            hash: 0,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, segmentation: &mut Vec<VolumeSegmenationBuffer>) {
        if segmentation.is_empty() {
            return;
        }

        self.changed = false;

        let mut hasher = DefaultHasher::new();
        for volume in segmentation.iter() {
            volume.name().hash(&mut hasher);
        }
        let hash = hasher.finish();

        self.changed = hash != self.hash;
        self.hash = hash;

        let mut index_to_remove: Option<usize> = None;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), self.changed)
            .show_header(ui, |ui| ui.heading("Segmentation"))
            .body(|ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    for (index, volume) in segmentation.iter_mut().enumerate() {
                        CollapsingState::load_with_default_open(
                            ui.ctx(),
                            volume.name().to_string().into(),
                            false,
                        )
                        .show_header(ui, |ui| {
                            if ui.button("🗑").clicked() {
                                index_to_remove = Some(index);
                                self.changed = true;
                            }

                            ui.label(volume.name());
                        })
                        .body(|ui| {
                            for setting in volume.settings_mut() {
                                CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    setting.name.to_string().into(),
                                    false,
                                )
                                .show_header(ui, |ui| {
                                    self.changed |= ui.checkbox(&mut setting.visible, "").changed();

                                    self.changed |=
                                        ui.color_edit_button_rgb(&mut setting.absorption).changed();

                                    self.changed |=
                                        ui.color_edit_button_rgb(&mut setting.scattering).changed();

                                    ui.label(setting.name.to_string());
                                })
                                .body(|_| {});
                            }
                        });
                    }
                })
            });

        if let Some(index) = index_to_remove {
            segmentation.remove(index);
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
