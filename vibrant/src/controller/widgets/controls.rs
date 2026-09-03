use egui::{RichText, Ui};

use crate::controller::icons;

pub struct ControlsWidget {}

impl Default for ControlsWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlsWidget {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&self, ui: &mut Ui) {
        ui.collapsing(
            RichText::new(format!("{} Controls", icons::regular::MOUSE)).heading(),
            |ui| {
                let actions = [
                    ("Rotate Camera", "Left Mouse Button"),
                    ("Pan Camera", "Right Mouse Button"),
                    ("Zoom Camera", "Mouse Wheel"),
                    ("Reset Camera", "Backspace"),
                    ("Rotate Light / Environment", "Shift + Left Mouse Button"),
                ];

                egui::Grid::new("my_grid").striped(true).show(ui, |ui| {
                    for (action, control) in actions {
                        ui.label(action);
                        ui.label(control);

                        let filler_width = (ui.available_width() - 1.0).max(0.0);
                        ui.add_sized([filler_width, 0.0], egui::Label::new(""));

                        ui.end_row();
                    }
                });
            },
        );
    }
}
