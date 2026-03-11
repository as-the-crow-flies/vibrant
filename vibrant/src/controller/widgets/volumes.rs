use std::{
    any::type_name,
    hash::{DefaultHasher, Hash, Hasher},
};

use egui::{collapsing_header::CollapsingState, ScrollArea, Ui};

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
                ScrollArea::new([false, true]).show(ui, |ui| {
                    for (index, volume) in volumes.iter_mut().enumerate() {
                        CollapsingState::load_with_default_open(
                            ui.ctx(),
                            volume.settings_mut().name.to_string().into(),
                            self.changed,
                        )
                        .show_header(ui, |ui| {
                            if ui.button("🗑").clicked() {
                                index_to_remove = Some(index);
                                self.changed = true;
                            }

                            self.changed |= ui
                                .checkbox(&mut volume.settings_mut().visible, "")
                                .changed();

                            self.changed |= ui
                                .color_edit_button_rgb(&mut volume.settings_mut().absorption)
                                .changed();

                            self.changed |= ui
                                .color_edit_button_rgb(&mut volume.settings_mut().scattering)
                                .changed();

                            ui.text_edit_singleline(&mut volume.settings_mut().name);
                        })
                        .body(|_| {});
                    }
                })
            });

        if let Some(index) = index_to_remove {
            volumes.remove(index);
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
