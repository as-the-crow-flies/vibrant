use std::{
    any::type_name,
    hash::{DefaultHasher, Hash, Hasher},
};

use egui::{collapsing_header::CollapsingState, Slider, Ui};

use crate::asset::hdri::HdriBuffer;

#[derive(Debug)]
pub struct HdriWidget {
    hash: u64,
    changed: bool,
}

impl HdriWidget {
    pub fn new() -> Self {
        Self {
            changed: false,
            hash: 0,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, hdri: &mut HdriBuffer) {
        self.changed = false;

        let mut hasher = DefaultHasher::new();
        hdri.name().hash(&mut hasher);
        let hash = hasher.finish();

        self.changed = hash != self.hash;
        self.hash = hash;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), self.changed)
            .show_header(ui, |ui| ui.heading("Environment Map"))
            .body(|ui| {
                if ui.button("Show/Hide").clicked() {
                    hdri.settings_mut().show = 1 - hdri.settings_mut().show;
                }

                self.changed |= ui
                    .add(Slider::new(&mut hdri.settings_mut().strength, 0.0..=2.0).text("Strength"))
                    .changed();

                self.changed |= ui
                    .add(Slider::new(&mut hdri.settings_mut().specular, 0.0..=1.0).text("Specular"))
                    .changed();

                self.changed |= ui
                    .add(Slider::new(&mut hdri.settings_mut().rotation, 0.0..=1.0).text("Rotation"))
                    .changed();
            });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
