use egui::{Grid, Ui};

use crate::controller::{components::UIComponents, icons};

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
        let camera_actions = [
            ("Rotate", "Left Mouse Button"),
            ("Pan", "Right Mouse Button"),
            ("Zoom", "Mouse Wheel"),
            ("Reset", "Backspace"),
        ];

        let light_actions = [("Rotate", "Shift + Left Mouse Button")];

        let camera_open = ui.is_new("Controls.Camera");
        ui.collapse(
            format!("{} Camera", icons::regular::VIDEO_CAMERA),
            camera_open,
            |ui| Self::actions_grid(ui, "ControlsCameraGrid", &camera_actions),
        );

        let light_open = ui.is_new("Controls.Light");
        ui.collapse(format!("{} Light", icons::regular::SUN), light_open, |ui| {
            Self::actions_grid(ui, "ControlsLightGrid", &light_actions)
        });
    }

    fn actions_grid(ui: &mut Ui, id: &str, actions: &[(&str, &str)]) {
        Grid::new(id)
            .num_columns(2)
            .spacing([40.0, ui.spacing().item_spacing.y])
            .show(ui, |ui| {
                for (action, control) in actions {
                    ui.label(*action);
                    ui.label(*control);
                    ui.end_row();
                }
            });
    }
}
