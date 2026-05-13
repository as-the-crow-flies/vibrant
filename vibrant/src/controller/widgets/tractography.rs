use egui::{Align, ComboBox, Layout, RichText, Ui};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    asset::{
        colormap::ColormapSelection,
        line::{LineBuffer, LineColorMode},
    },
    controller::{components::UIComponents, widgets::util::UiResponseExtensions},
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
    const LINE_COLOR_MODE_HELP: &str = "
Tangent: Tangent RGB Coloring
Color: Fixed Bundle Coloring
Scalar: Coloring according to corresponding .tsf file
        ";

    pub fn new() -> Self {
        Self {
            visible: true,
            changed: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, lines: &mut Option<LineBuffer>) {
        self.changed = false;

        let open = if let Some(lines) = lines {
            ui.is_new(
                &lines
                    .settings()
                    .iter()
                    .map(|settings| &settings.name)
                    .join(" "),
            )
        } else {
            false
        };

        let response = ui
            .collapse(RichText::new("Tractography").heading(), open, |ui| {
                if let Some(lines) = lines {
                    for line in lines.settings_mut() {
                        ui.frame(|ui| {
                            let response = ui.collapse(&line.name, false, |_| {});

                            ui.inline(&response, |ui| {
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
                                    })
                                    .response
                                    .help("Line Color Mode", Self::LINE_COLOR_MODE_HELP);

                                if line.color_mode == LineColorMode::Color {
                                    ui.color_edit_button_srgb(&mut line.color)
                                        .on_hover_text("Choose Bundle Color")
                                        .track(self);
                                }

                                if line.color_mode == LineColorMode::Scalar {
                                    ComboBox::from_id_salt(format!("{}_LineColormap", line.name))
                                        .selected_text(format!("{:?}", line.colormap))
                                        .show_ui(ui, |ui| {
                                            for map in ColormapSelection::iter() {
                                                ui.selectable_value(
                                                    &mut line.colormap,
                                                    map,
                                                    format!("{:?}", map),
                                                );
                                            }
                                        })
                                        .response
                                        .on_hover_text("Choose .tsf Colormap");
                                }
                            });
                        });
                    }
                }
            })
            .help(
                "Tractography",
                "Open one or more .tck files to render tractograms.",
            );

        ui.inline(&response, |ui| {
            if let Some(lines) = lines {
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
                        })
                        .response
                        .help("Line Color Mode", Self::LINE_COLOR_MODE_HELP);

                    if color_mode_clicked {
                        let color_mode = lines.settings_global_mut().color_mode;

                        for line in lines.settings_mut() {
                            line.color_mode = color_mode;
                        }

                        self.track();
                    };
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
