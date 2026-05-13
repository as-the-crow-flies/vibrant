use std::iter::zip;

use egui::{Grid, RichText, Ui};
use itertools::Itertools;

use crate::{
    asset::volume_mask::{VolumeMaskBuffer, VolumeMaskSettings},
    controller::{components::UIComponents, widgets::util::UiResponseExtensions},
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct MasksWidget {
    changed: bool,
}

impl Tracked for MasksWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl MasksWidget {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn show(&mut self, ui: &mut Ui, masks: &mut Vec<VolumeMaskBuffer>) {
        self.changed = false;

        let mut masks = masks
            .iter_mut()
            .filter(|mask| mask.settings().name != "None")
            .collect_vec();

        let open = masks
            .iter()
            .map(|mask| ui.is_new(&mask.settings().name))
            .collect_vec();

        ui.collapse(
            RichText::new("Masks").heading(),
            open.iter().any(|&x| x),
            |ui| {
                for (mask, open) in zip(masks.iter_mut(), open) {
                    let settings = mask.settings_mut();

                    ui.frame(|ui| {
                        let title = settings.name.clone();
                        let response = ui.collapse(title, open, |ui| {
                            self.show_mask(ui, settings);
                        });

                        ui.inline(&response, |ui| {
                            ui.toggle_inverted(&mut settings.inverted).track(self);
                            ui.toggle_visible(&mut settings.visible).track(self);
                        });
                    });
                }
            },
        ).help("Volume Masking", "Open *mask*.nii.gz files to load masks. Then apply the mask to one or more NIfTI volumes.\nTwo types of masks are supported: Binary masks and Signed Distance Masks.");
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    fn show_mask(&mut self, ui: &mut Ui, mask: &mut VolumeMaskSettings) {
        if mask.binary {
            ui.horizontal(|ui| {
                ui.label("Blend")
                    .on_hover_text("Blend between fully and partially masked");

                ui.slider(&mut mask.offset, 0.0..=1.0).track(self);
            });
        } else {
            Grid::new("MaskSettings").num_columns(2).show(ui, |ui| {
                ui.label("Offset")
                    .on_hover_text("Signed Distance Mask Offset");

                ui.slider(&mut mask.offset, -0.5..=0.5).track(self);
                ui.end_row();

                ui.label("Smoothing")
                    .on_hover_text("Signed Distance Mask Smoothing");
                ui.slider(&mut mask.width, 0.01..=0.1).track(self);
                ui.end_row();
            });
        }
    }
}
