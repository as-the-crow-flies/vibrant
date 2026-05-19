use egui::{ComboBox, Grid, RichText, Ui};
use itertools::Itertools;

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

        let name = hdri.texture().name().to_owned();

        let names = hdri
            .textures()
            .iter()
            .map(|texture| texture.name().to_owned())
            .collect_vec();

        ui.collapse(RichText::new("Environment").heading(), false, |ui| {
            Grid::new("EnvironmentMapGrid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Environment");
                    ComboBox::from_id_salt("EnvironmentMapSelect")
                        .selected_text(name)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for (index, name) in names.iter().enumerate() {
                                ui.selectable_value(&mut hdri.index, index, name)
                                    .track(self);
                            }
                        });
                    ui.end_row();

                    ui.label("Strength");
                    ui.slider(&mut hdri.settings_mut().strength, 0.0..=9.99)
                        .track(self);
                    ui.end_row();

                    ui.label("Specular");
                    ui.slider(&mut hdri.settings_mut().specular, 0.0..=2.0)
                        .track(self);
                    ui.end_row();

                    ui.label("Rotation");
                    ui.slider(&mut hdri.settings_mut().rotation, 0.0..=1.0)
                        .track(self);
                    ui.end_row();
                });
        })
        .help(
            "Environment Textures",
            "Add realistic lighting by loading an environment texture.\n
            Click open to load an .exr file (e.g. from http://polyhaven.com)",
        );
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
