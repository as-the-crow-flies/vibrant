use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Slider, Ui};

use crate::{
    asset::hdri::HdriBuffer,
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct HdriWidget {
    changed: bool,
}

impl Tracked for HdriWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl HdriWidget {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn show(&mut self, ui: &mut Ui, hdri: &mut HdriBuffer) {
        self.changed = false;

        if hdri.name() == "Default" {
            return;
        }

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), true)
            .show_header(ui, |ui| ui.heading("Environment Map"))
            .body(|ui| {
                ui.add(Slider::new(&mut hdri.settings_mut().strength, 0.0..=2.0).text("Strength"))
                    .track(self);

                ui.add(Slider::new(&mut hdri.settings_mut().specular, 0.0..=1.0).text("Specular"))
                    .track(self);

                ui.add(Slider::new(&mut hdri.settings_mut().rotation, 0.0..=1.0).text("Rotation"))
                    .track(self);
            });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
