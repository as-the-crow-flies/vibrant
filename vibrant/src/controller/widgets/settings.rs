use egui::{ComboBox, Grid, Ui};

use crate::{
    controller::{camera::Camera, components::UIComponents, settings::Settings},
    util::{ResponseExtentions, Tracked},
};

#[derive(Debug)]
pub struct SettingsWidget {
    changed: bool,
}

impl Tracked for SettingsWidget {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl SettingsWidget {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn show(&mut self, ui: &mut Ui, settings: &mut Settings, camera: &mut Camera) {
        self.changed = false;

        ui.collapse("Camera", false, |ui| {
            Grid::new("CameraSettings").num_columns(2).show(ui, |ui| {
                ui.label("Field of View")
                    .on_hover_text("Camera Field of View");
                ui.slider(&mut camera.fov, 0.4..=1.0).track(self);
                ui.end_row();
            });
        });

        ui.collapsing("Tractography", |ui| {
            Grid::new("TractographySettings")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Resolution").on_hover_text(
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

                    ui.label("Memory (MB)").on_hover_text("Tractography Acceleration Structure Memory Usage.\nAutoselected on native platforms.");
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

                    ui.label("Radius").on_hover_text("Tractography Line Radius");
                    ui.slider(&mut settings.radius, 0.0..=1.0).track(self);
                    ui.end_row();

                    ui.label("Ambient").on_hover_text("Ambient Lighting Strength");
                    ui.slider(&mut settings.ambient_light, 0.0..=3.0)
                        .track(self);
                    ui.end_row();

                    ui.label("Sun").on_hover_text("Directional Lighting Strength");
                    ui.slider(&mut settings.direct_light, 0.0..=3.0).track(self);
                    ui.end_row();

                    ui.label("Opacity").on_hover_text("Tractography Opacity");
                    ui.slider(&mut settings.alpha, 0.01..=1.0).track(self);
                    ui.end_row();
                });
        });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
