use std::{
    any::type_name,
    hash::{DefaultHasher, Hash, Hasher},
};

use egui::{collapsing_header::CollapsingState, Slider, Ui};

use crate::asset::volume_mask::VolumeMaskBuffer;

#[derive(Debug)]
pub struct MaskWidget {
    hash: u64,
    changed: bool,
}

impl MaskWidget {
    pub fn new() -> Self {
        Self {
            changed: false,
            hash: 0,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, mask: &mut VolumeMaskBuffer) {
        self.changed = false;

        let mut hasher = DefaultHasher::new();
        mask.settings().hash(&mut hasher);
        let hash = hasher.finish();

        self.changed = hash != self.hash;
        self.hash = hash;

        let settings = mask.settings_mut();

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), self.changed)
            .show_header(ui, |ui| {
                ui.checkbox(&mut settings.visible, "");
                ui.heading("Mask");
            })
            .body(|ui| {
                ui.spacing_mut().slider_width = 300.0;
                ui.checkbox(&mut settings.invert, "Invert");
                ui.add(Slider::new(&mut settings.offset, -1.0..=1.0).text("Offset"));
                ui.add(Slider::new(&mut settings.width, 0.001..=0.2).text("Width"));
            });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
