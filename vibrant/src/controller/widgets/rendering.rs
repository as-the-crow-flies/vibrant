use egui::{ComboBox, Grid, RichText, Ui};
use itertools::Itertools;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    asset::hdri::HdriBuffer,
    controller::{
        camera::Camera, components::UIComponents, icons, settings::Settings,
        widgets::util::UiResponseExtensions,
    },
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum RadianceMethod {
    Linear,
    Gaussian,
}

#[derive(Debug)]
pub struct RenderingWidget {
    changed: bool,
    method: RadianceMethod,
    resolution: u32,
    lobes: u32,
    /// While true, HDR strength follows the display's headroom limit. Cleared
    /// when the user drags the slider below the max, re-set if they drag it
    /// back up.
    hdr_headroom_auto: bool,
}

impl Tracked for RenderingWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl RenderingWidget {
    pub fn new() -> Self {
        Self {
            changed: false,
            method: RadianceMethod::Gaussian,
            resolution: 4,
            lobes: 16,
            hdr_headroom_auto: true,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        hdri: &mut HdriBuffer,
        camera: &mut Camera,
        settings: &mut Settings,
        // `Some(limit)` = HDR available + the max peak the display can drive now.
        hdr_headroom_limit: Option<f32>,
    ) {
        self.changed = false;

        let name = hdri.texture().name().to_owned();

        let names = hdri
            .textures()
            .iter()
            .map(|texture| texture.name().to_owned())
            .collect_vec();

        ui.collapse(
            RichText::new(format!("{} Rendering", icons::regular::LIGHTBULB)).heading(),
            false,
            |ui| {
                let camera_open = ui.is_new("Rendering.Camera");
                ui.collapse(
                    format!("{} Camera", icons::regular::VIDEO_CAMERA),
                    camera_open,
                    |ui| {
                        Grid::new("CameraSettings").num_columns(2).show(ui, |ui| {
                            ui.label("Field of View")
                                .on_hover_text("Camera Field of View");
                            ui.slider(&mut camera.fov, 0.4..=1.0).track(self);
                            ui.end_row();
                        });
                    },
                );

                let environment_open = ui.is_new("Rendering.Environment");
                ui.collapse(
                    format!("{} Environment", icons::regular::PANORAMA),
                    environment_open,
                    |ui| {
                        Grid::new("EnvironmentMapGrid")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Map");
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
                                ui.slider(&mut hdri.settings_mut().strength, 0.0..=8.0)
                                    .track(self);
                                ui.end_row();

                                ui.label("Specular");
                                ui.slider(&mut hdri.settings_mut().specular, 0.0..=1.0)
                                    .track(self);
                                ui.end_row();

                                ui.label("Roughness");
                                ui.slider(&mut hdri.settings_mut().roughness, 0.0..=1.0)
                                    .track(self);
                                ui.end_row();
                            });
                    },
                )
                .help(
                    "Environment Textures",
                    "Add realistic lighting by loading an environment texture.\n
                    Click open to load an .exr file (e.g. from http://polyhaven.com)",
                );

                let display_open = ui.is_new("Rendering.Display");
                ui.collapse(
                    format!("{} Display", icons::regular::MONITOR),
                    display_open,
                    |ui| {
                        Grid::new("DisplaySettings").num_columns(2).show(ui, |ui| {
                            ui.label("HDR Display").on_hover_text(
                                "Present to an HDR (scRGB) display instead of tone \
                                 mapping to SDR with ACES. Disabled when the current \
                                 display or GPU can't present HDR.",
                            );
                            ui.add_enabled(
                                hdr_headroom_limit.is_some(),
                                egui::Checkbox::new(&mut settings.hdr, "Enabled"),
                            )
                            .track(self);
                            ui.end_row();

                            if let Some(limit) = hdr_headroom_limit {
                                // Default to the display's max; keep following it
                                // until the user drags the slider down.
                                if self.hdr_headroom_auto {
                                    settings.hdr_headroom = limit;
                                }
                                settings.hdr_headroom =
                                    settings.hdr_headroom.clamp(1.0, limit);

                                ui.label("HDR Strength").on_hover_text(
                                    "Peak brightness as a multiple of SDR white. \
                                     Capped at what the display reports it can \
                                     drive.",
                                );
                                let changed = ui
                                    .add_enabled(
                                        settings.hdr,
                                        egui::Slider::new(
                                            &mut settings.hdr_headroom,
                                            1.0..=limit,
                                        )
                                        .fixed_decimals(2),
                                    )
                                    .track(self)
                                    .changed();
                                if changed {
                                    self.hdr_headroom_auto =
                                        settings.hdr_headroom >= limit;
                                }
                                ui.end_row();
                            }
                        });
                    },
                );

                ui.collapse(
                    format!("{} Advanced", icons::regular::SLIDERS),
                    false,
                    |ui| {
                        Grid::new("RadianceSettings").num_columns(2).show(ui, |ui| {
                            ui.label("Lighting Method");
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
                                        .track(self);
                                    }
                                });
                            ui.end_row();

                            ui.label("Lighting Resolution");
                            ComboBox::from_id_salt("Resolution")
                                .selected_text(format!("{:?}", self.resolution))
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for setting in [1, 2, 4, 8, 16] {
                                        ui.selectable_value(
                                            &mut self.resolution,
                                            setting,
                                            format!("{}", setting),
                                        )
                                        .track(self);
                                    }
                                });
                            ui.end_row();

                            ui.label("Lighting Lobes");
                            ComboBox::from_id_salt("Lobes")
                                .selected_text(format!("{:?}", self.lobes))
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for setting in [8, 16, 32] {
                                        ui.selectable_value(
                                            &mut self.lobes,
                                            setting,
                                            format!("{}", setting),
                                        )
                                        .track(self);
                                    }
                                });
                            ui.end_row();

                            ui.label("Tractography Resolution").on_hover_text(
                                "Voxel Resolution for Tractography Ray Tracing.\nHigher values result in sharper shadows, but may be slower.",
                            );
                            ComboBox::from_id_salt("Voxel Resolution")
                                .selected_text(format!("{:?}", settings.volume))
                                .width(ui.available_width())
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

                            ui.label("Memory (MB)").on_hover_text("Tractography Acceleration Structure Memory Usage.\nAutoselected.");
                            ComboBox::from_id_salt("Memory")
                                .selected_text(format!("{:?}", settings.index_buffer_size))
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for power in 6u32..13 {
                                        ui.selectable_value(
                                            &mut settings.index_buffer_size,
                                            2u32.pow(power),
                                            format!("{}", 2u32.pow(power)),
                                        )
                                        .track(self);
                                    }
                                });
                            ui.end_row();
                        });
                    },
                );
            },
        );
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

    pub fn lobes(&self) -> u32 {
        self.lobes
    }
}
