use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Grid, Ui};

use crate::{
    asset::hdri::HdriBuffer,
    controller::{components::UIComponents, widgets::util::UiResponseExtensions},
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

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), true)
            .show_header(ui, |ui| {
                ui.heading("Environment").help(
                    "Environment Textures",
                    "Add realistic lighting by loading an environment texture.\n
                    Click open to load an .exr file (e.g. from http://polyhaven.com)",
                )
            })
            .body(|ui| {
                if hdri.name() == "Default" {
                    return;
                }

                Grid::new("EnvironmentMapGrid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Strength");
                        ui.slider(&mut hdri.settings_mut().strength, 0.0..=2.0)
                            .track(self);
                        ui.end_row();

                        ui.label("Specular");
                        ui.slider(&mut hdri.settings_mut().specular, 0.0..=1.0)
                            .track(self);
                        ui.end_row();

                        ui.label("Rotation");
                        ui.slider(&mut hdri.settings_mut().rotation, 0.0..=1.0)
                            .track(self);
                        ui.end_row();
                    });
            });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
