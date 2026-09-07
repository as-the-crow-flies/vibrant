use egui::{Align, ComboBox, Frame, Grid, Layout, Margin, ScrollArea, Ui};
use strum::IntoEnumIterator;

use crate::{
    asset::{
        colormap::ColormapSelection,
        line::{LineBuffer, LineColorMode},
    },
    controller::{
        components::UIComponents,
        icons,
        settings::{RenderMode, Settings},
        widgets::util::UiResponseExtensions,
    },
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

impl Default for TractographyWidget {
    fn default() -> Self {
        Self::new()
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

    pub fn show(&mut self, ui: &mut Ui, lines: &mut Option<LineBuffer>, settings: &mut Settings) {
        self.changed = false;

        let Some(lines) = lines else {
            ui.vertical_centered(|ui| {
                ui.weak("Open .tck files using the [📂 open] button");
            });
            return;
        };

        // Closed by default (`open: false`, same as every other collapsible
        // here) and rendered ahead of the tract list's own `ScrollArea`
        // below, so it stays put at the top instead of scrolling away with
        // the tracts.
        ui.collapse(format!("{} Settings", icons::regular::GEAR), false, |ui| {
            Grid::new("TractographySettings")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Render Mode");
                    ComboBox::from_id_salt("TractographyRenderMode")
                        .selected_text(format!("{}", settings.render_mode))
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.render_mode,
                                RenderMode::Combined,
                                format!("{}", RenderMode::Combined),
                            )
                            .track(self);
                            ui.selectable_value(
                                &mut settings.render_mode,
                                RenderMode::XRay,
                                format!("{}", RenderMode::XRay),
                            )
                            .track(self);
                        });
                    ui.end_row();

                    // `settings.radius` is in voxels of the `settings.volume` grid,
                    // which spans the tractogram's largest extent; scale by the mm
                    // width of one such voxel so the slider reads in millimetres.
                    if let Some(mm_per_unit) = (settings.volume > 0)
                        .then(|| lines.bounds().scale().max_element() / settings.volume as f32)
                    {
                        ui.label("Tract Radius")
                            .on_hover_text("Tractography line radius in mm");
                        let mut radius_mm = settings.radius * mm_per_unit;
                        if ui.slider(&mut radius_mm, 0.0..=2.0).track(self).changed() {
                            settings.radius = radius_mm / mm_per_unit;
                        }
                        ui.end_row();
                    }
                });
        });

        ui.separator();

        // Right-inset to match the 6px `inner_margin` that `ui.frame` (used
        // below, per line) applies on every side - otherwise this row's
        // right-aligned buttons sit 6px further right than the per-line ones.
        Frame::NONE
            .inner_margin(Margin {
                right: 6,
                ..Margin::ZERO
            })
            .show(ui, |ui| {
                // `Align::Min` (not `Center`): the dock tab body's `ui` is
                // pre-expanded to the leaf's full height, so a `Center`-aligned
                // horizontal layout would seed its row at the vertical middle of
                // that whole height instead of right after the previous content.
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
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

                    if ui
                        .selectable_label(settings.line_crop, icons::regular::CROP)
                        .on_hover_text("Let the slicing / clipping planes cut the tractography")
                        .clicked()
                    {
                        settings.line_crop = !settings.line_crop;

                        for line in lines.settings_mut() {
                            line.crop = settings.line_crop;
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
            });

        ScrollArea::new([false, true]).show(ui, |ui| {
            for line in lines.settings_mut() {
                ui.frame(|ui| {
                    let response = ui.collapse(&line.name, false, |_| {});

                    ui.inline(&response, |ui| {
                        ui.toggle_visible(&mut line.visible).track(self);

                        ui.toggle(
                            icons::regular::CROP,
                            "Let the slicing / clipping planes cut this tract",
                            &mut line.crop,
                        )
                        .track(self);

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
        });
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
