use egui::{ComboBox, Grid, Ui};
use strum::{EnumIter, IntoEnumIterator};

use crate::util::{ResponseExtentions, Tracked};

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum RadianceMethod {
    Octahedral,
    Holographic,
}

#[derive(Debug)]
pub struct RadianceWidget {
    method: RadianceMethod,
    resolution: u32,
    changed: bool,
}

impl Tracked for RadianceWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl RadianceWidget {
    pub fn new() -> Self {
        Self {
            changed: false,
            method: RadianceMethod::Octahedral,
            resolution: 4,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.changed = false;

        ui.collapsing("Radiance", |ui| {
            Grid::new("RadianceSettings").num_columns(2).show(ui, |ui| {
                ui.label("Method");
                ComboBox::from_id_salt("Method")
                    .selected_text(format!("{:?}", self.method))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for setting in RadianceMethod::iter() {
                            ui.selectable_value(
                                &mut self.method,
                                setting,
                                format!("{:?}", setting),
                            )
                            .track(self)
                            .changed();
                        }
                    });
                ui.end_row();

                ui.label("Resolution");
                ComboBox::from_id_salt("Resolution")
                    .selected_text(format!("{:?}", self.resolution))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for setting in [1, 2, 4, 8] {
                            ui.selectable_value(
                                &mut self.resolution,
                                setting,
                                format!("{}", setting),
                            )
                            .track(self)
                            .changed();
                        }
                    });
                ui.end_row();
            });
        });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn method(&self) -> RadianceMethod {
        self.method
    }

    pub fn resolution(&self) -> u32 {
        self.resolution
    }
}
