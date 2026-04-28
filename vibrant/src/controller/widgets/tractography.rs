use egui::{collapsing_header::CollapsingState, Align, ComboBox, Layout, RichText, Ui};
use strum::IntoEnumIterator;

use crate::{
    asset::{
        colormap::ColormapSelection,
        line::{LineBuffer, LineColorMode},
    },
    controller::components::UIComponents,
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct TractographyWidget {
    changed: bool,
    visible: bool,
}

impl Tracked for TractographyWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl TractographyWidget {
    pub fn new() -> Self {
        Self {
            visible: true,
            changed: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, lines: &mut LineBuffer) {
        self.changed = false;

        CollapsingState::load_with_default_open(ui.ctx(), "Tractography".into(), true)
            .show_header(ui, |ui| {
                ui.heading("Tractography");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(7.0);

                    if ui
                        .toggle_visible(&mut lines.settings_global_mut().visible)
                        .clicked()
                    {
                        let visible = lines.settings_global_mut().visible;

                        for line in lines.settings_mut() {
                            line.visible = visible;
                        }

                        self.track();
                    }

                    let mut color_mode_clicked = false;

                    ComboBox::from_id_salt("LineGlobalColorMode")
                        .selected_text(format!("{:?}", lines.settings_global_mut().color_mode))
                        .show_ui(ui, |ui| {
                            for mode in LineColorMode::iter() {
                                color_mode_clicked |= ui
                                    .selectable_value(
                                        &mut lines.settings_global_mut().color_mode,
                                        mode,
                                        format!("{:?}", mode),
                                    )
                                    .clicked();
                            }
                        });

                    if color_mode_clicked {
                        let color_mode = lines.settings_global_mut().color_mode;

                        for line in lines.settings_mut() {
                            line.color_mode = color_mode;
                        }

                        self.track();
                    };
                });
            })
            .body(|ui| {
                for line in lines.settings_mut() {
                    let id = ui.make_persistent_id(&line.name);

                    ui.frame(|ui| {
                        CollapsingState::load_with_default_open(ui.ctx(), id, false)
                            .show_header(ui, |ui| {
                                ui.label(RichText::new(&line.name).strong());

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.toggle_visible(&mut line.visible).track(self);

                                    ComboBox::from_id_salt(format!("{}_LineColorMode", line.name))
                                        .selected_text(format!("{:?}", line.color_mode))
                                        .show_ui(ui, |ui| {
                                            for mode in LineColorMode::iter() {
                                                ui.selectable_value(
                                                    &mut line.color_mode,
                                                    mode,
                                                    format!("{:?}", mode),
                                                );
                                            }
                                        });

                                    if line.color_mode == LineColorMode::Color {
                                        ui.color_edit_button_srgb(&mut line.color).track(self);
                                    }

                                    if line.color_mode == LineColorMode::Scalar {
                                        ComboBox::from_id_salt(format!(
                                            "{}_LineColormap",
                                            line.name
                                        ))
                                        .selected_text(format!("{:?}", line.colormap))
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                for map in ColormapSelection::iter() {
                                                    ui.selectable_value(
                                                        &mut line.colormap,
                                                        map,
                                                        format!("{:?}", map),
                                                    );
                                                }
                                            },
                                        );
                                    }
                                });
                            })
                            .body(|_| {});
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
