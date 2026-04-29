use egui::{ComboBox, Grid, Ui};
use egui_double_slider::DoubleSlider;

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

        ui.collapsing("Camera", |ui| {
            Grid::new("CameraSettings").num_columns(2).show(ui, |ui| {
                ui.label("Field of View");
                ui.slider(&mut camera.fov, 0.1..=3.0).track(self);
                ui.end_row();
            });
        });

        ui.collapsing("Tractography", |ui| {
            Grid::new("TractographySettings")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Resolution");
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

                    ui.label("Memory (MB)");
                    ComboBox::from_id_salt("Memory")
                        .selected_text(format!("{:?}", settings.fragment_list_size))
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for power in 6u32..13 {
                                ui.selectable_value(
                                    &mut settings.fragment_list_size,
                                    2u32.pow(power),
                                    format!("{}", 2u32.pow(power)),
                                )
                                .track(self);
                            }
                        });
                    ui.end_row();

                    ui.label("Radius");
                    ui.slider(&mut settings.radius, 0.0..=1.0).track(self);
                    ui.end_row();

                    ui.label("Alpha");
                    ui.slider(&mut settings.alpha, 0.01..=1.0).track(self);
                    ui.end_row();

                    ui.label("Crop");
                    ui.add(
                        DoubleSlider::new(
                            &mut settings.crop_start,
                            &mut settings.crop_end,
                            0.0..=1.0,
                        )
                        .width(ui.available_width())
                        .separation_distance(0.01),
                    )
                    .track(self);
                    ui.end_row();
                });
        });
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
