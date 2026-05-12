use std::any::type_name;

use egui::{collapsing_header::CollapsingState, Align, Grid, Layout, RichText, Ui};

use crate::{
    asset::volume_mask::VolumeMaskBuffer,
    controller::{components::UIComponents, widgets::util::UiResponseExtensions},
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct MasksWidget {
    changed: bool,
}

impl Tracked for MasksWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl MasksWidget {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn show(&mut self, ui: &mut Ui, masks: &mut Vec<VolumeMaskBuffer>) {
        self.changed = false;

        CollapsingState::load_with_default_open(ui.ctx(), type_name::<Self>().into(), true)
            .show_header(ui, |ui| {
                ui.heading("Masks")
                    .help("Volume Masking", "Open *mask*.nii.gz files to load masks. Then apply the mask to one or more NIfTI volumes.\nTwo types of masks are supported: Binary masks and Signed Distance Masks.");
            })
            .body(|ui| {
                for mask in masks.iter_mut() {
                    let settings = mask.settings_mut();

                    // Don't show default mask
                    if settings.name == "None" {
                        continue;
                    }

                    ui.frame(|ui| {
                        CollapsingState::load_with_default_open(
                            ui.ctx(),
                            settings.name.to_string().into(),
                            true,
                        )
                        .show_header(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&settings.name).strong());

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.toggle_inverted(&mut settings.inverted).track(self);
                                    ui.toggle_visible(&mut settings.visible).track(self);
                                });
                            });
                        })
                        .body(|ui| {
                            if settings.binary {
                                ui.horizontal(|ui| {
                                    ui.label("Blend").on_hover_text("Blend between fully and partially masked");
                                    ui.scope(|ui| {
                                        ui.slider(&mut settings.offset, 0.0..=1.0).track(self);
                                    });
                                });
                            } else {
                                Grid::new("MaskSettings").num_columns(2).show(ui, |ui| {
                                    ui.label("Offset").on_hover_text("Signed Distance Mask Offset");

                                    ui.slider(&mut settings.offset, -0.5..=0.5).track(self);
                                    ui.end_row();

                                    ui.label("Smoothing").on_hover_text("Signed Distance Mask Smoothing");
                                    ui.slider(&mut settings.width, 0.01..=0.1).track(self);
                                    ui.end_row();
                                });
                            }
                        });
                    });
                }
            });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
