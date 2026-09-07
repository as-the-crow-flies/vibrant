use std::iter::zip;

use egui::{Grid, Ui};
use itertools::Itertools;

use crate::{
    asset::volume_mask::{VolumeMaskBuffer, VolumeMaskSettings},
    controller::components::UIComponents,
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

impl Default for MasksWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl MasksWidget {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn show(&mut self, ui: &mut Ui, masks: &mut [VolumeMaskBuffer]) {
        self.changed = false;

        let mut masks = masks
            .iter_mut()
            .filter(|mask| mask.settings().name != "None")
            .collect_vec();

        if masks.is_empty() {
            ui.vertical_centered(|ui| {
                ui.weak("Open *mask*.nii.gz files using the [📂 open] button");
            });
            return;
        }

        let open = masks
            .iter()
            .map(|mask| ui.is_new(&mask.settings().name))
            .collect_vec();

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
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    fn show_mask(&mut self, ui: &mut Ui, mask: &mut VolumeMaskSettings) {
        if mask.binary {
            ui.horizontal(|ui| {
                ui.label("Blend").on_hover_text(
                    "How strongly this mask is applied. 0 leaves the volume \
                     untouched; 1 clips it fully to the mask.",
                );

                ui.slider(&mut mask.offset, 0.0..=1.0).track(self);
            });
        } else {
            Grid::new("MaskSettings").num_columns(2).show(ui, |ui| {
                ui.label("Offset").on_hover_text(
                    "Grow or shrink the masked region. Negative values erode \
                     inward, positive values dilate outward.",
                );

                ui.slider(&mut mask.offset, -1.0..=1.0).track(self);
                ui.end_row();

                ui.label("Smoothing").on_hover_text(
                    "Softness of the mask edge. Higher values fade the boundary \
                     over a wider band instead of a hard cut.",
                );
                ui.slider(&mut mask.width, 0.1..=1.0).track(self);
                ui.end_row();
            });
        }
    }
}
