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

    pub fn show(&mut self, ui: &mut Ui, segmentation: &mut [VolumeSegmenationBuffer]) {
        self.changed = false;

        let mut hasher = DefaultHasher::new();
        for volume in segmentation.iter() {
            volume.name().hash(&mut hasher);
        }
        let hash = hasher.finish();

        self.changed = hash != self.hash;
        self.hash = hash;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), false)
            .show_header(ui, |ui| ui.heading("Volume Segmentation"))
            .body(|ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    for volume in segmentation {
                        for setting in volume.settings() {
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
                    }
                })
            });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
