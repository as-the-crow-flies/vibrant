use egui::{ComboBox, Grid, Ui};
use itertools::Itertools;

use crate::{
    asset::hdri::HdriBuffer,
    controller::{
        camera::Camera, components::UIComponents, icons, settings::Settings,
        widgets::util::UiResponseExtensions,
    },
    surface::accumulate::AccumulationStatus,
    util::{ResponseExtentions, Tracked},
};

/// Selectable lightmap (radiance probe grid) resolutions, in probes along the
/// volume's longest axis.
const LIGHTMAP_RESOLUTIONS: [u32; 7] = [64, 96, 128, 160, 192, 224, 256];

/// Selectable lighting lobe counts.
const LOBES: [u32; 3] = [8, 16, 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quality {
    Low,
    Medium,
    High,
    Custom,
}

impl Quality {
    const ALL: [Quality; 4] = [
        Quality::Low,
        Quality::Medium,
        Quality::High,
        Quality::Custom,
    ];

    /// `(lightmap resolution, lobes)` for a preset; `None` for `Custom`.
    fn preset(self) -> Option<(u32, u32)> {
        match self {
            Quality::Low => Some((64, 8)),
            Quality::Medium => Some((128, 16)),
            Quality::High => Some((256, 32)),
            Quality::Custom => None,
        }
    }
}

#[derive(Debug)]
pub struct RenderingWidget {
    changed: bool,
    quality: Quality,
    lightmap_resolution: u32,
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

impl Default for RenderingWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderingWidget {
    pub fn new() -> Self {
        Self {
            changed: false,
            quality: Quality::Low,
            lightmap_resolution: 64,
            lobes: 8,
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
        // Live accumulation progress from the renderer.
        accumulation: AccumulationStatus,
    ) {
        self.changed = false;

        let name = hdri.texture().name().to_owned();

        let names = hdri
            .textures()
            .iter()
            .map(|texture| texture.name().to_owned())
            .collect_vec();

        let camera_open = ui.is_new("Rendering.Camera");
        ui.collapse(
            format!("{} Camera", icons::regular::VIDEO_CAMERA),
            camera_open,
            |ui| {
                Grid::new("CameraSettings").num_columns(2).show(ui, |ui| {
                    ui.label("Field of View")
                        .on_hover_text("Camera Field of View");
                    ui.slider(&mut camera.fov, 0.2..=1.0).track(self);
                    ui.end_row();

                    ui.label("HDR Display").on_hover_text(
                        "Present to an HDR (scRGB) display instead of tone \
                         mapping to SDR with ACES. Disabled when the current \
                         display or GPU can't present HDR.",
                    );
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            hdr_headroom_limit.is_some(),
                            egui::Checkbox::new(&mut settings.hdr, ""),
                        )
                        .track(self);

                        if let Some(limit) = hdr_headroom_limit {
                            // Default to the display's max; keep following it
                            // until the user drags the slider down.
                            if self.hdr_headroom_auto {
                                settings.hdr_headroom = limit;
                            }
                            settings.hdr_headroom = settings.hdr_headroom.clamp(1.0, limit);

                            let changed = ui
                                .add_enabled_ui(settings.hdr, |ui| {
                                    ui.slider(&mut settings.hdr_headroom, 1.0..=limit)
                                        .on_hover_text(
                                            "Peak brightness as a multiple of SDR white. \
                                             Capped at what the display reports it can \
                                             drive.",
                                        )
                                        .track(self)
                                        .changed()
                                })
                                .inner;
                            if changed {
                                self.hdr_headroom_auto = settings.hdr_headroom >= limit;
                            }
                        }
                    });
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
                    });
            },
        )
        .help(
            "Environment Textures",
            "Add realistic lighting by loading an environment texture.\n
            Click open to load an .exr file (e.g. from http://polyhaven.com)",
        );

        ui.collapse(format!("{} Quality", icons::regular::GAUGE), false, |ui| {
            Grid::new("RadianceSettings").num_columns(2).show(ui, |ui| {
                ui.label("Preset");
                ComboBox::from_id_salt("QualityPreset")
                    .selected_text(format!("{:?}", self.quality))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for preset in Quality::ALL {
                            if ui
                                .selectable_value(&mut self.quality, preset, format!("{preset:?}"))
                                .changed()
                            {
                                if let Some((resolution, lobes)) = preset.preset() {
                                    self.lightmap_resolution = resolution;
                                    self.lobes = lobes;
                                }
                                self.track();
                            }
                        }
                    });
                ui.end_row();

                let custom = self.quality == Quality::Custom;

                ui.label("Lightmap Resolution");
                ui.add_enabled_ui(custom, |ui| {
                    ComboBox::from_id_salt("Resolution")
                        .selected_text(format!("{}", self.lightmap_resolution))
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for setting in LIGHTMAP_RESOLUTIONS {
                                ui.selectable_value(
                                    &mut self.lightmap_resolution,
                                    setting,
                                    format!("{setting}"),
                                )
                                .track(self);
                            }
                        });
                });
                ui.end_row();

                ui.label("Lighting Lobes");
                ui.add_enabled_ui(custom, |ui| {
                    ComboBox::from_id_salt("Lobes")
                        .selected_text(format!("{}", self.lobes))
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for setting in LOBES {
                                ui.selectable_value(&mut self.lobes, setting, format!("{setting}"))
                                    .track(self);
                            }
                        });
                });
                ui.end_row();
            });
        });

        ui.collapse(
            format!("{} Accumulation", icons::regular::STACK),
            false,
            |ui| {
                let AccumulationStatus { samples, converged } = accumulation;

                Grid::new("AccumulationSettings")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Enabled").on_hover_text(
                            "Blend successive frames while the view is \
                             still, reducing noise and anti-aliasing \
                             edges. Rendering pauses once the image \
                             converges, saving power.",
                        );
                        ui.checkbox(&mut settings.accumulate, "").track(self);
                        ui.end_row();

                        ui.label("Max Samples")
                            .on_hover_text("Stop accumulating after this many frames.");
                        ui.add_enabled(
                            settings.accumulate,
                            egui::Slider::new(&mut settings.max_samples, 1..=4096)
                                .logarithmic(true),
                        )
                        .track(self);
                        ui.end_row();

                        ui.label("Noise Threshold").on_hover_text(
                            "Stop early once the mean per-pixel change \
                             between samples drops below this. Larger = \
                             stop sooner (noisier).",
                        );
                        ui.add_enabled(
                            settings.accumulate,
                            egui::Slider::new(&mut settings.noise_threshold, 1.0e-4..=1.0e-2)
                                .logarithmic(true)
                                .custom_formatter(|n, _| format!("{n:.1e}")),
                        )
                        .track(self);
                        ui.end_row();

                        ui.label("Status");
                        ui.label(if !settings.accumulate {
                            "off".to_owned()
                        } else if converged {
                            format!("converged ({samples} samples)")
                        } else {
                            format!("{samples} / {}", settings.max_samples.max(1))
                        });
                        ui.end_row();
                    });
            },
        );
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Target probe count along the volume's longest axis for the radiance
    /// lightmap; the other axes scale to preserve aspect ratio.
    pub fn lightmap_resolution(&self) -> u32 {
        self.lightmap_resolution
    }

    /// Override the lightmap resolution. For headless callers (benches) that
    /// never open the widget.
    pub fn set_lightmap_resolution(&mut self, resolution: u32) {
        self.quality = Quality::Custom;
        self.lightmap_resolution = resolution.max(1);
    }

    pub fn lobes(&self) -> u32 {
        self.lobes
    }
}
