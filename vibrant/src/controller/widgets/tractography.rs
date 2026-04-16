use std::hash::{DefaultHasher, Hash, Hasher};

use egui::{collapsing_header::CollapsingState, Slider, Ui};
use itertools::Itertools;

use crate::{asset::line::LineBuffer, controller::ternary_checkbox};

#[derive(Debug)]
pub struct TractographyWidget {
    hash: u64,
    changed: bool,
    visible: bool,
}

impl TractographyWidget {
    pub fn new() -> Self {
        Self {
            visible: true,
            changed: false,
            hash: 0,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, lines: &mut LineBuffer) {
        self.changed = false;

        let mut hasher = DefaultHasher::new();
        for line in lines.settings() {
            line.hash(&mut hasher);
        }
        let hash = hasher.finish();

        self.changed = hash != self.hash;
        self.hash = hash;

        CollapsingState::load_with_default_open(ui.ctx(), "Tractography".into(), false)
            .show_header(ui, |ui| {
                ui.checkbox(&mut self.visible, "");
                ui.heading("Tractography");
            })
            .body(|ui| {
                lines.settings_global().selected = lines
                    .settings()
                    .iter()
                    .map(|settings| settings.selected)
                    .all_equal_value()
                    .ok();

                lines.settings_global().visible = lines
                    .settings()
                    .iter()
                    .map(|settings| settings.visible)
                    .all_equal_value()
                    .ok();

                CollapsingState::load_with_default_open(ui.ctx(), "Line".into(), false)
                    .show_header(ui, |ui| {
                        if let Some(visible) =
                            ternary_checkbox(ui, lines.settings_global().visible, "👁")
                        {
                            lines.settings_global().visible = Some(visible);

                            for line in lines.settings() {
                                line.visible = visible;
                            }
                        }

                        if let Some(color_visible) = ternary_checkbox(
                            ui,
                            Some(lines.settings_global().color_visible),
                            "   🎨   ",
                        ) {
                            lines.settings_global().color_visible = color_visible;

                            for line in lines.settings() {
                                line.color_visible = color_visible
                            }
                        }
                    })
                    .body(|_| {});

                for line in lines.settings() {
                    let id = ui.make_persistent_id(&line.name);
                    CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
                            ui.toggle_value(&mut line.visible, "👁");
                            ui.color_edit_button_srgb(&mut line.color);
                            ui.label(&line.name);
                        })
                        .body(|ui| {
                            ui.add(Slider::new(&mut line.crop_start, 0.0..=1.0).text("Crop Start"));
                            ui.add(Slider::new(&mut line.crop_end, 0.0..=1.0).text("Crop End"));
                        });
                }
            });
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
