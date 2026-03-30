pub mod camera;
pub mod event;
pub mod light;
pub mod segment;
pub mod settings;
pub mod state;

use std::time::Instant;

use camera::Camera;
use egui::{
    Align, Button, Color32, ComboBox, Frame, Label, Layout, Margin, ScrollArea, Sense, SidePanel, Slider, Ui, collapsing_header::CollapsingState, debug_text::print
};
use event::Event;
use itertools::Itertools;
use light::Light;
use settings::Settings;
use state::ControllerState;
use winit::dpi::PhysicalSize;

use crate::{
    asset::{Asset, line },
    asset::line::LineSettings,
    controller::{
        segment::Segment,
        settings::{LineDisplayMode, LineVoxelizationMode},
    },
    file::FileStage,
};

#[derive(Debug, Clone)]
pub enum Layer {
    Line(String),
    Group(Vec<String>, LineSettings)
}

impl Layer {
    fn contains_name(&self, name: &str) -> bool {
        match self {
            Layer::Line(line_name) => line_name == name,
            Layer::Group(names, _) => names.contains(&name.to_string()),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Location {
    layer_index: usize,
    group_index: usize, //remove later
    group_item_index: usize
}

#[derive(Debug)]
pub struct Controller {
    state: ControllerState,
    camera: Camera,
    light: Light,
    segment: Segment,
    settings: Settings,
    time: Instant,

    show_left_side_panel: bool,
    show_right_side_panel: bool,

    layers: Vec<Layer>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: ControllerState::default(),
            camera: Camera::new(),
            light: Light::default(),
            segment: Segment::new(),
            settings: Settings::new(),
            time: Instant::now(),

            show_left_side_panel: false,
            show_right_side_panel: false,

            layers: Vec::new(),
        }
    }

    pub fn update_line_assets (&mut self, asset: &mut Asset) {
        println!("Updating line assets");

        let mut new_line_settings = Vec::new();

        let Some(line_buffer) = asset.line.as_mut() else {
            return;
        };

        for layer in &self.layers {
            match layer {
                Layer::Line(name) => {
                    if let Some(line) = line_buffer
                        .settings()
                        .iter_mut()
                        .find(|line| line.name == *name) {
                        new_line_settings.push(line.clone());
                    }
                },
                Layer::Group(group_lines, _) => {
                    for line_name in group_lines {
                        if let Some(line) = line_buffer
                            .settings()
                            .iter_mut()
                            .find(|line| line.name == *line_name) {
                            new_line_settings.push(line.clone());
                        }
                    }
                }
            }
        }

        line_buffer.clear_settings();
        line_buffer.set_settings(new_line_settings.clone());
    }

    pub fn event(&mut self, event: Event) {
        self.state = self.state.update(event);

        self.camera.update(&self.state);
        self.light.update(&self.state);
    }

    pub fn ui(&mut self, ctx: &egui::Context, asset: &mut Asset, _dt: f32) {
        if let Some(lines) = &mut asset.line {
            for line in lines.settings() {
                if !self.layers.iter().any(|layer| layer.contains_name(&line.name)) {
                    let layer = Layer::Line(line.name.clone());
                    println!("Adding layer for line {}", line.name);
                    self.layers.push(layer);
                }
            }
        }

        egui::TopBottomPanel::top("TopBottomPanel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("⚙ settings")
                    .on_hover_text("Open settings panel")
                    .clicked()
                {
                    self.show_left_side_panel = !self.show_left_side_panel;
                }

                #[cfg(not(target_arch = "wasm32"))]
                if ui
                    .button("📷 screenshot")
                    .on_hover_text("Take screenshot with transparent background")
                    .clicked()
                {
                    FileStage::save();
                }

                ui.take_available_width();

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("☰ layers").clicked() {
                        self.show_right_side_panel = !self.show_right_side_panel
                    }

                    if ui
                        .button("📂 open")
                        .on_hover_text("Open .tck/.obj files")
                        .clicked()
                    {
                        FileStage::load();
                    }
                });
            });
        });

        egui::SidePanel::left("SidePanelLeft").show_animated(
            ctx,
            self.show_left_side_panel,
            |ui| {
                egui::TopBottomPanel::top("top_panel")
                    .frame(Frame {
                        outer_margin: Margin {
                            left: 5,
                            right: 5,
                            top: 5,
                            bottom: 10,
                        },
                        inner_margin: Margin::ZERO,
                        ..Default::default()
                    })
                    .show_inside(ui, |ui| {
                        ui.heading("Rendering");
                        ui.separator();

                        ComboBox::from_label("Display Mode")
                            .selected_text(format!("{:?}", self.settings.display))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.display,
                                    LineDisplayMode::Geometry,
                                    "Geometry",
                                );
                                ui.selectable_value(
                                    &mut self.settings.display,
                                    LineDisplayMode::Volume,
                                    "Volume",
                                );
                            });

                        ComboBox::from_label("Voxelization Mode")
                            .selected_text(format!("{:?}", self.settings.voxelization))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.voxelization,
                                    LineVoxelizationMode::Tube,
                                    "Tube",
                                );
                                ui.selectable_value(
                                    &mut self.settings.voxelization,
                                    LineVoxelizationMode::Box,
                                    "Box",
                                );
                                ui.selectable_value(
                                    &mut self.settings.voxelization,
                                    LineVoxelizationMode::Line,
                                    "Line",
                                );
                            });

                        ComboBox::from_label("Voxel Resolution")
                            .selected_text(format!("{:?}", self.settings.volume))
                            .show_ui(ui, |ui| {
                                for power in 5u32..10 {
                                    ui.selectable_value(
                                        &mut self.settings.volume,
                                        2u32.pow(power),
                                        format!("{}", 2u32.pow(power)),
                                    );
                                }
                            });

                        ui.separator();
                        ui.label("Appearance");
                        ui.separator();

                        ui.add(Slider::new(&mut self.camera.fov, 0.1..=3.0));
                        ui.add(
                            Slider::new(&mut self.settings.radius, 0.01..=1.0)
                                .text("Streamline Radius"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.lighting, 0.0..=1.0).text("Lighting"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.direct_light, 0.0..=3.0)
                                .text("Ambient/Shadow"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.tangent_color, 0.0..=2.0)
                                .text("Tangent Color"),
                        );
                        ui.add(Slider::new(&mut self.settings.alpha, 0.01..=1.0).text("Alpha"));
                        ui.add(
                            Slider::new(&mut self.settings.smoothing, 0.0..=1.0).text("Smoothing"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.crop_start, 0.0..=1.0)
                                .text("Crop Start"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.crop_end, 0.0..=1.0).text("Crop End"),
                        );

                        if ui
                            .add(
                                Slider::new(&mut self.settings.crop_middle, 0.0..=0.5)
                                    .text("Crop Middle"),
                            )
                            .changed()
                        {
                            self.settings.crop_start = 0.5 - self.settings().crop_middle;
                            self.settings.crop_end = 0.5 + self.settings().crop_middle;
                        }

                        ui.add(
                            Slider::new(&mut self.settings.crop_x_start, -0.5..=0.5)
                                .step_by(1.0 / self.settings.volume as f64)
                                .text("Crop X Start"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.crop_x_end, -0.5..=0.5)
                                .step_by(1.0 / self.settings.volume as f64)
                                .text("Crop X End"),
                        );

                        ui.add(
                            Slider::new(&mut self.settings.crop_y_start, -0.5..=0.5)
                                .step_by(1.0 / self.settings.volume as f64)
                                .text("Crop Y Start"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.crop_y_end, -0.5..=0.5)
                                .step_by(1.0 / self.settings.volume as f64)
                                .text("Crop Y End"),
                        );

                        ui.add(
                            Slider::new(&mut self.settings.crop_z_start, -0.5..=0.5)
                                .step_by(1.0 / self.settings.volume as f64)
                                .text("Crop Z Start"),
                        );
                        ui.add(
                            Slider::new(&mut self.settings.crop_z_end, -0.5..=0.5)
                                .step_by(1.0 / self.settings.volume as f64)
                                .text("Crop Z End"),
                        );

                        ui.add(Slider::new(&mut self.settings.plane, 0.0..=1.0).text("plane"));

                        ui.add(
                            Slider::new(&mut self.settings.workgroups, 1..=128)
                                .text("# Workgroups"),
                        );
                        ui.separator();
                        ui.label("Post Processing");
                        ui.separator();
                        ui.checkbox(&mut self.settings.bloom, "Bloom");
                        ui.add_enabled(
                            self.settings.bloom,
                            Slider::new(&mut self.settings.bloom_threshold, 0.0..=1.0)
                                .text("Bloom Threshold"),
                        );
                        ui.add_enabled(
                            self.settings.bloom,
                            Slider::new(&mut self.settings.bloom_intensity, 0.0..=3.0)
                                .text("Bloom Intensity"),
                        );
                        ui.add_enabled(
                            self.settings.bloom,
                            Slider::new(&mut self.settings.bloom_spread, 1.0..=5.0)
                                .text("Bloom Spread"),
                        );
                    });

                egui::TopBottomPanel::bottom("bottom_panel")
                    .frame(Frame {
                        outer_margin: Margin {
                            left: 5,
                            right: 5,
                            top: 5,
                            bottom: 10,
                        },
                        inner_margin: Margin::ZERO,
                        ..Default::default()
                    })
                    .show_inside(ui, |ui| {
                        ui.heading("Controls");
                        ui.separator();

                        egui::Grid::new("my_grid")
                            .min_col_width(100.0)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Rotate Camera");
                                ui.label("Left Mouse Button");
                                ui.end_row();

                                ui.label("Pan Camera");
                                ui.label("Right Mouse Button");
                                ui.end_row();

                                ui.label("Zoom Camera");
                                ui.label("Mouse Wheel");
                                ui.end_row();

                                ui.label("Reset Camera");
                                ui.label("Backspace");
                                ui.end_row();

                                ui.label("Rotate Light");
                                ui.label("Shift + Left Mouse Button");
                                ui.end_row();
                            });
                    });
            },
        );

        SidePanel::right("SidePanelRight")
            .min_width(300.0)
            .show_animated(ctx, self.show_right_side_panel, |ui| {
                CollapsingState::load_with_default_open(ui.ctx(), "Tractography".into(), false)
                    .show_header(ui, |ui| ui.heading("Tractography"))
                    .body(|ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            if let Some(lines) = &mut asset.line {
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

                                CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    "Line".into(),
                                    false,
                                )
                                .show_header(ui, |ui| {
                                    if let Some(visible) =
                                        ternary_checkbox(ui, lines.settings_global().visible, "👁")
                                    {
                                        lines.settings_global().visible = Some(visible);

                                        for line in lines.settings() {
                                            line.visible = visible;
                                        }
                                    }

                                    if let Some(color_visible) = ternary_checkbox(
                                        ui,
                                        Some(lines.settings_global().color_visible),
                                        "   🎨   ",
                                    ) {
                                        lines.settings_global().color_visible = color_visible;

                                        for line in lines.settings() {
                                            line.color_visible = color_visible
                                        }
                                    }
                                })
                                .body(|_| {});

                                self.tractography_layers(ui, asset);
                            }
                        });
                    });

                CollapsingState::load_with_default_open(ui.ctx(), "Volumes".into(), false)
                    .show_header(ui, |ui| ui.heading("Volumes"))
                    .body(|ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            for volume in &mut asset.volumes {
                                let id = ui.make_persistent_id(&volume.name());
                                CollapsingState::load_with_default_open(ui.ctx(), id, false)
                                    .show_header(ui, |ui| {
                                        ui.label(volume.name());
                                    })
                                    .body(|_| {});
                            }
                        });
                    });
            });
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn light(&self) -> &Light {
        &self.light
    }

    pub fn segment(&self) -> &Segment {
        &self.segment
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.settings.width = size.width;
        self.settings.height = size.height;
    }

    pub fn time(&self) -> f32 {
        Instant::now().duration_since(self.time).as_secs_f32()
    }

    pub fn tractography_layers (&mut self, ui: &mut Ui, asset: &mut Asset) {
        let mut from = None;
        let mut to = None;

        let mut group_index = 0;

        for (index, layer) in self.layers.iter_mut().enumerate() {
            match layer {
                Layer::Line(name) => {
                    let line = asset.line.as_mut().unwrap().settings().iter_mut().find(|line| line.name == *name).unwrap();
                    let id = ui.make_persistent_id(&line.name);
                    CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
                            ui.toggle_value(&mut line.visible, "👁");
                            ui.color_edit_button_srgb(&mut line.color);

                            drop_zone(ui, &line.name, id, Location { group_index: 0, layer_index: index,group_item_index: 0 }, &mut from, &mut to);
                        })
                        .body(|ui| {
                            ui.add(
                                Slider::new(&mut line.crop_start, 0.0..=1.0)
                                    .text("Crop Start"),
                            );
                            ui.add(
                                Slider::new(&mut line.crop_end, 0.0..=1.0)
                                    .text("Crop End"),
                            );

                            if ui.add(
                                Button::new("Create Group")
                            ).clicked() {
                                let new_layer = Layer::Group(vec![line.name.clone()], line.clone());
                                *layer = new_layer;
                            };
                        });
                },
                Layer::Group(group_lines, group_settings) => {
                    group_index += 1;

                    let id = ui.make_persistent_id(format!("Group{}", index));
                    CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
                            ui.toggle_value(&mut group_settings.visible, "👁");
                            ui.color_edit_button_srgb(&mut group_settings.color);
                            ui.label(format!("Group {}", index));

                            //drop_zone(ui, id, Location { group_index: group_index, layer_index: index, group_item_index: 0 }, &mut from, &mut to);
                        })
                        .body(|ui| {
                            for (group_item_index, line_name) in group_lines.iter().enumerate() {
                                let line_in_group_id = ui.make_persistent_id(format!("Group{}Group_Item{}", group_index, group_item_index));
                                drop_zone(ui, line_name, id, Location { group_index: group_index, layer_index: index, group_item_index: group_item_index }, &mut from, &mut to);
                            }

                            let response1 = ui.add(
                        Slider::new(&mut group_settings.crop_start, 0.0..=1.0)
                                    .text("Crop Start"),
                            );
                            let response2 = ui.add(
                                Slider::new(&mut group_settings.crop_end, 0.0..=1.0)
                                    .text("Crop End"),
                            );

                            if response1.changed() || response2.changed() {
                                for line_name in group_lines.iter() {
                                    println!("Updating crop settings for line {} in group {}", line_name, index);
                                    
                                    if let Some(line) = asset.line.as_mut().unwrap().settings().iter_mut().find(|line| line.name == *line_name) {
                                        line.crop_start = group_settings.crop_start;
                                        line.crop_end = group_settings.crop_end;
                                    }
                                }
                            }
                        });
                },
            }
        }

        if let (Some(from), Some(mut to)) = (from, to) {
            println!("Move line from group {} line {} to group {} line {}", from.group_index, from.layer_index, to.group_index, to.layer_index);

            let from_layer = self.get_line_by_indexes(from);
            let to_layer = self.get_line_by_indexes(to);
            println!("from_layer: {:?}", from_layer);
            println!("to_layer: {:?}", to_layer);

            if let Some(name) = from_layer {
                let mut layers = self.layers.clone();

                layers = self.remove_line_from_layers_by_location(layers, from);

                if to.group_index != 0 {                  
                    if from.layer_index < to.layer_index {
                        to.layer_index -= 1;
                    } else if from.layer_index == to.layer_index && from.group_item_index < to.group_item_index {
                        to.group_item_index -= 1;
                    }
                }

                layers = self.insert_line_into_layers_by_name_and_indexes(layers, &name, to);

                self.layers = layers;
            }

            Controller::update_line_assets(self, asset);
        }
    }

    fn get_line_by_indexes (&self, location: Location) -> Option<String> {
        let layer = &self.layers[location.layer_index];

        match layer {
            Layer::Line(line_name) => return Some(line_name.clone()),
            Layer::Group(names, _) => {
                let name = names[location.group_item_index].clone();
                return Some(name)
            } ,
            _ => return None
        }

    }

    fn remove_line_from_layers_by_location (&self, mut layers: Vec<Layer>, location: Location) -> Vec<Layer> {
        println!("Removing line from layers at group index {} and line index {}", location.group_index, location.layer_index);

        let layer = &mut layers[location.layer_index];
        match layer {
            Layer::Line(line_name) => {
                layers.remove(location.layer_index);
            },
            Layer::Group(group_lines, _) => {
                let line_name = group_lines.remove(location.group_item_index);
            }
        }

        layers
    }
      
    fn insert_line_into_layers_by_name_and_indexes (&self, mut layers: Vec<Layer>, name: &str, location: Location) -> Vec<Layer> {
        if layers.len() == location.layer_index {
            layers.push(Layer::Line(name.to_string()));
            return layers;
        }

        let layer = &mut layers[location.layer_index];
        match layer {
            Layer::Line(_) => {
                layers.insert(location.layer_index, Layer::Line(name.to_string()));
            },
            Layer::Group(group_lines, group_settings) => {
                group_lines.insert(location.group_item_index, name.to_string());
            }
        }

        layers
    }
}

fn ternary_checkbox(ui: &mut Ui, input: Option<bool>, text: &str) -> Option<bool> {
    let mut checked = input.unwrap_or_default();

    ui.toggle_value(&mut checked, text)
        .clicked()
        .then_some(checked)
}

fn drop_zone(ui: &mut Ui, item_name: &String, item_id: egui::Id, item_location: Location, from: &mut Option<Location>, to: &mut Option<Location>) {
    let frame = Frame::default().inner_margin(4.0);
    let (_, dropped_payload) = ui.dnd_drop_zone::<Location, ()>(frame, |ui| {
        let item_id = egui::Id::new(("drag_and_drop", item_location.group_index, item_location.layer_index, item_location.group_item_index));

        let row_idx = item_location.layer_index;

        let response = ui
            .dnd_drag_source(item_id, item_location.clone(), |ui| {
                ui.label(item_name);
            })
            .response;
        
        // Detect drops onto this item:
        if let (Some(pointer), Some(hovered_payload)) = (
            ui.input(|i| i.pointer.interact_pos()), 
            response.dnd_hover_payload::<Location>(),
        ) {
            println!("Drop detected");

            let rect = response.rect;

            let line_index = item_location.layer_index;
            let group_index = item_location.group_index;

            //https://github.com/emilk/egui/blob/main/crates/egui_demo_lib/src/demo/drag_and_drop.rs
            // Preview insertion:
            let stroke = egui::Stroke::new(1.0, Color32::WHITE);
            let insert_row_idx = 
            if hovered_payload.group_index == group_index && hovered_payload.layer_index == line_index {
                // We are dragged onto ourselves
                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                row_idx
            } else if pointer.y < rect.center().y {
                // Above us
                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                
                if row_idx > 0 {
                    row_idx - 1
                } else {
                    row_idx
                }
            } else {
                // Below us
                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                row_idx + 1
            };

            if let Some(dragged_payload) = response.dnd_release_payload::<Location>() {
                println!("Drop released");

                *from = Some((*dragged_payload).clone());
                *to = Some(item_location);

                println!("Move line from group {} line {} to group {} line {}", dragged_payload.group_index, dragged_payload.layer_index, group_index, insert_row_idx);
                println!("from: {:?}", from.as_ref().unwrap());
                println!("to: {:?}", to.as_ref().unwrap());
            }
        }
    });
}
