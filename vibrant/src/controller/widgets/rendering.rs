use egui::{ComboBox, Grid, Ui};
use itertools::Itertools;

use crate::{
    asset::{hdri::HdriBuffer, line::LineBuffer},
    controller::{
        camera::Camera,
        components::UIComponents,
        icons,
        settings::{RenderMode, Settings},
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
            Quality::Medium => Some((96, 16)),
            Quality::High => Some((128, 32)),
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
            quality: Quality::Medium,
            lightmap_resolution: 96,
            lobes: 16,
            hdr_headroom_auto: true,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        hdri: &mut HdriBuffer,
        lines: Option<&LineBuffer>,
        camera: &mut Camera,
        settings: &mut Settings,
        // `Some(limit)` = HDR available + the max peak the display can drive now.
        hdr_headroom_limit: Option<f32>,
        // Live accumulation progress from the renderer.
        accumulation: AccumulationStatus,
    ) {
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
                    ui.label("Field of View").on_hover_text(
                        "Camera field of view. Low values flatten perspective \
                         (closer to an orthographic view); high values \
                         exaggerate depth.",
                    );
                    ui.slider(&mut camera.fov, 0.2..=1.0).track(self);
                    ui.end_row();

                    ui.label("HDR Display").on_hover_text(
                        "Send the image to an HDR display for brighter \
                         highlights, instead of tone-mapping for a standard \
                         display. Unavailable if the display or GPU can't show \
                         HDR.",
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
                                            "Peak brightness of the HDR image, as a \
                                             multiple of standard-display white. \
                                             Limited by what the display reports it \
                                             can produce.",
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
                        ui.label("Map")
                            .on_hover_text("360° environment image used to light the scene.");
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

                        ui.label("Strength").on_hover_text(
                            "Brightness of the environment lighting. Raise it \
                             for a brighter scene and stronger highlights.",
                        );
                        ui.slider(&mut hdri.settings_mut().strength, 0.0..=8.0)
                            .track(self);
                        ui.end_row();
                    });
            },
        )
        .help(
            "Environment lighting",
            "The scene is lit by a 360° environment image. Pick a built-in map, \
             or use Open to load your own .exr (e.g. from polyhaven.com). \
             Shift + drag in the view to rotate the lighting.",
        );

        let volumes_open = ui.is_new("Rendering.Volumes");
        ui.collapse(
            format!("{} Volumes", icons::regular::BRAIN),
            volumes_open,
            |ui| {
                Grid::new("VolumeRenderSettings")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Specular").on_hover_text(
                            "Strength of mirror-like highlights on dense structures \
                         under environment lighting. 0 = matte, 1 = glossy.",
                        );
                        ui.slider(&mut hdri.settings_mut().specular, 0.0..=1.0)
                            .track(self);
                        ui.end_row();

                        ui.label("Roughness").on_hover_text(
                            "How spread-out the specular highlights are. Low = tight, \
                         wet-looking reflections; high = a soft, broad sheen.",
                        );
                        ui.slider(&mut hdri.settings_mut().roughness, 0.0..=1.0)
                            .track(self);
                        ui.end_row();
                    });
            },
        );

        let tractography_open = ui.is_new("Rendering.Tractography");
        ui.collapse(
            format!("{} Tractography", icons::regular::PATH),
            tractography_open,
            |ui| {
                Grid::new("TractographyRenderSettings")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Render Mode").help(
                            "Render Mode",
                            "How bundles are combined with the volumes.\n\n\
                             Combined — bundles and volume share one lighting pass \
                             and occlude each other, so tracts pass convincingly \
                             behind anatomy.\n\
                             Overlay — bundles are drawn on top of the volume and \
                             stay fully visible, like a see-through schematic.",
                        );
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
                                    RenderMode::Overlay,
                                    format!("{}", RenderMode::Overlay),
                                )
                                .track(self);
                            });
                        ui.end_row();

                        // `settings.radius` is in voxels of the `settings.volume` grid,
                        // which spans the tractogram's largest extent; scale by the mm
                        // width of one such voxel so the slider reads in millimetres.
                        if let Some(mm_per_unit) =
                            lines.filter(|_| settings.volume > 0).map(|lines| {
                                lines.bounds().scale().max_element() / settings.volume as f32
                            })
                        {
                            ui.label("Tract Radius").on_hover_text(
                                "Rendered radius of each streamline, in millimetres.",
                            );
                            let mut radius_mm = settings.radius * mm_per_unit;
                            if ui.slider(&mut radius_mm, 0.0..=2.0).track(self).changed() {
                                settings.radius = radius_mm / mm_per_unit;
                            }
                            ui.end_row();
                        }
                    });
            },
        );

        ui.collapse(format!("{} Quality", icons::regular::GAUGE), false, |ui| {
            Grid::new("RadianceSettings").num_columns(2).show(ui, |ui| {
                ui.label("Preset").on_hover_text(
                    "Overall lighting quality. Higher settings give smoother, \
                     more accurate light and shadows but render more slowly. \
                     Pick \"Custom\" to set the values below yourself.",
                );
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

                ui.label("Lightmap Resolution").on_hover_text(
                    "Detail of the lighting grid across the scene. Higher values \
                     resolve finer shadow and lighting detail, at the cost of \
                     speed.",
                );
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

                ui.label("Lighting Lobes").on_hover_text(
                    "How many directions each point uses to gather light. More \
                     lobes give smoother, less noisy lighting but render more \
                     slowly.",
                );
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
                            "While the view is still, blend successive frames to \
                             reduce noise and smooth edges. Rendering pauses \
                             automatically once the image stops changing.",
                        );
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut settings.accumulate, "").track(self);
                            ui.label(if !settings.accumulate {
                                "off".to_owned()
                            } else if converged {
                                format!("converged ({samples} samples)")
                            } else {
                                format!("{samples} / {}", settings.max_samples.max(1))
                            });
                        });
                        ui.end_row();
                    });
            },
        );
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn reset_changed(&mut self) {
        self.changed = false;
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

    /// Override the lobe count. For headless callers (benches) that never open
    /// the widget.
    pub fn set_lobes(&mut self, lobes: u32) {
        self.quality = Quality::Custom;
        self.lobes = lobes;
    }
}
