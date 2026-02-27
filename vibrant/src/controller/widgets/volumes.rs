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

    pub fn show(&mut self, ui: &mut Ui, volumes: &mut [VolumeFractionBuffer]) {
        self.changed = false;

        let mut hasher = DefaultHasher::new();
        for volume in volumes.iter() {
            volume.settings().name.hash(&mut hasher);
        }
        let hash = hasher.finish();

        self.changed = hash != self.hash;
        self.hash = hash;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), false)
            .show_header(ui, |ui| ui.heading("Volume Fractions"))
            .body(|ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    for volume in volumes {
                        CollapsingState::load_with_default_open(
                            ui.ctx(),
                            volume.settings_mut().name.to_string().into(),
                            false,
                        )
                        .show_header(ui, |ui| {
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
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
