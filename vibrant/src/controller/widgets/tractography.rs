use egui::{collapsing_header::CollapsingState, ComboBox, Grid, Slider, Ui};
use egui_double_slider::DoubleSlider;
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    asset::{
        colormap::ColormapSelection,
        line::{LineBuffer, LineColorMode},
    },
    controller::{settings::Settings, ternary_checkbox},
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

    pub fn show(&mut self, ui: &mut Ui, settings: &mut Settings, lines: &mut LineBuffer) {
        self.changed = false;

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

        CollapsingState::load_with_default_open(ui.ctx(), "Tractography".into(), true)
            .show_header(ui, |ui| {
                ui.checkbox(&mut self.visible, "").track(self);
                ui.heading("Tractography");
            })
            .body(|ui| {
                CollapsingState::load_with_default_open(ui.ctx(), "Line".into(), false)
                    .show_header(ui, |ui| {
                        if let Some(visible) =
                            ternary_checkbox(ui, lines.settings_global().visible, "👁")
                        {
                            lines.settings_global().visible = Some(visible);

                            for line in lines.settings() {
                                line.visible = visible;
                            }

                            self.track();
                        }
                    })
                    .body(|ui| {
                        Grid::new("TractographySettings").show(ui, |ui| {
                            ui.label("Resolution");
                            ComboBox::from_id_salt("Voxel Resolution")
                                .selected_text(format!("{:?}", settings.volume))
                                .show_ui(ui, |ui| {
                                    for power in 5u32..10 {
                                        ui.selectable_value(
                                            &mut settings.volume,
                                            2u32.pow(power),
                                            format!("{}", 2u32.pow(power)),
                                        )
                                        .track(self);
                                    }
                                });

                            ui.end_row();

                            ui.label("Radius");
                            ui.add(Slider::new(&mut settings.radius, 0.0..=1.0))
                                .track(self);
                            ui.end_row();

                            ui.label("Alpha");
                            ui.add(Slider::new(&mut settings.alpha, 0.01..=1.0))
                                .track(self);
                            ui.end_row();

                            ui.label("Crop");
                            ui.add(
                                DoubleSlider::new(
                                    &mut settings.crop_start,
                                    &mut settings.crop_end,
                                    0.0..=1.0,
                                )
                                .width(300.0)
                                .separation_distance(0.01),
                            )
                            .track(self);
                            ui.end_row();
                        });
                    });

                for line in lines.settings() {
                    let id = ui.make_persistent_id(&line.name);
                    CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
                            ui.toggle_value(&mut line.visible, "👁").track(self);

                            ui.text_edit_singleline(&mut line.name).track(self);

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
                                    });
                            }
                        })
                        .body(|_| {});
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
