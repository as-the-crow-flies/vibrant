use egui::{
    Align, Align2, CollapsingHeader, Color32, ComboBox, FontId, Frame, Grid, Id, InnerResponse,
    Layout, Mesh, Pos2, Rect, Sense, Shape, Stroke, TextStyle, Ui, UiBuilder, Vec2,
};
use strum::IntoEnumIterator;

use crate::{
    asset::{
        material::{Material, MaterialNode, MaterialPreset},
        volume_fraction::{MaterialNodeBuffer, VolumeFractionSettings},
    },
    controller::components::UIComponents,
    util::{ResponseExtentions, Tracked},
};

pub struct TransferFunctionEditor {
    next_picker_id: u64,
    changed: bool,
}

impl Tracked for TransferFunctionEditor {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl TransferFunctionEditor {
    pub fn new() -> Self {
        Self {
            next_picker_id: 2,
            changed: false,
        }
    }

    fn next_picker_id(&mut self) -> u64 {
        let id = self.next_picker_id;
        self.next_picker_id += 1;
        id
    }

    pub fn show(&mut self, ui: &mut Ui, volume: &mut VolumeFractionSettings) {
        self.changed = false;

        let spacing = ui.spacing().item_spacing;

        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            self.toolbar(ui, volume);

            volume
                .nodes
                .sort_by(|a, b| a.position.total_cmp(&b.position));

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                let histogram_rect = self.histogram(ui, volume).inner;

                self.pickers(ui, volume, histogram_rect);
            });
        });

        let mut clicked = None;
        let volume_id = volume.id;

        for (index, node) in volume.nodes.iter_mut().enumerate() {
            if Self::settings(self, ui, volume_id, node, spacing, index + 1) {
                clicked = Some(index);
            }
        }

        if let Some(clicked) = clicked {
            for (index, node) in volume.nodes.iter_mut().enumerate() {
                node.selected = index == clicked;
            }
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, volume: &mut VolumeFractionSettings) {
        ui.with_layout(Layout::top_down(Align::Max), |ui| {
            if ui
                .icon_button(
                    "➕",
                    volume.nodes.len() < MaterialNodeBuffer::MAX_NODES,
                    false,
                )
                .on_hover_text("Add color stop")
                .track(self)
                .clicked()
            {
                let position = volume
                    .nodes
                    .iter()
                    .find(|node| node.selected)
                    .map(|node| (node.position + 0.1).clamp(0.0, 1.0))
                    .unwrap_or(0.5);

                for node in volume.nodes.iter_mut() {
                    node.selected = false;
                }

                volume.nodes.push(MaterialNode {
                    id: self.next_picker_id(),
                    selected: true,
                    position,
                    preset: MaterialPreset::Custom,
                    material: Material::default(),
                });
            }

            let selected = volume.nodes.iter().position(|node| node.selected);

            if ui
                .icon_button("➖", selected.is_some(), false)
                .on_hover_text("Remove selected color stop")
                .track(self)
                .clicked()
            {
                if let Some(index) = selected {
                    volume.nodes.remove(index);

                    if let Some(last) = volume.nodes.last_mut() {
                        last.selected = true;
                    }
                }
            }
        });
    }

    fn histogram(&self, ui: &mut Ui, volume: &mut VolumeFractionSettings) -> InnerResponse<Rect> {
        Frame::canvas(&ui.style())
            .fill(ui.visuals().extreme_bg_color)
            .show(ui, |ui| {
                let size = Vec2::new(ui.available_width(), 4.0 * ui.spacing().interact_size.y);
                let rect = Rect::from_min_size(ui.cursor().min, size);

                ui.allocate_space(size);
                ui.set_clip_rect(rect);

                let width = rect.width() / 256.0;

                let mut mesh = Mesh::default();

                for index in 0..=256 {
                    let t = index as f32 / 256.0;
                    let x = rect.min.x + t * rect.width();
                    let color = self.gradient(volume, t);

                    mesh.colored_vertex(Pos2::new(x, rect.min.y), color);
                    mesh.colored_vertex(Pos2::new(x, rect.max.y), color);

                    if index > 0 {
                        let base = (index as u32 - 1) * 2;
                        mesh.add_triangle(base, base + 1, base + 2);
                        mesh.add_triangle(base + 1, base + 3, base + 2);
                    }
                }

                ui.painter().add(mesh);

                let histogram = volume.histogram.clone();
                let values = histogram.values();
                let bar_color = Color32::from_white_alpha(90);

                // Sample heights at bin centers and extend to the canvas edges, so
                // the area fill is a continuous piecewise-linear curve through the
                // bin values rather than a staircase of flat-topped bars.
                let mut area = Mesh::default();

                for index in 0..=257 {
                    let (x, value) = match index {
                        0 => (rect.min.x, values[0]),
                        257 => (rect.max.x, values[255]),
                        _ => {
                            let bin = index - 1;
                            (rect.min.x + (bin as f32 + 0.5) * width, values[bin])
                        }
                    };

                    let y_top = rect.max.y - rect.height() * value;

                    area.colored_vertex(Pos2::new(x, y_top), bar_color);
                    area.colored_vertex(Pos2::new(x, rect.max.y), bar_color);

                    if index > 0 {
                        let base = (index as u32 - 1) * 2;
                        area.add_triangle(base, base + 1, base + 2);
                        area.add_triangle(base + 1, base + 3, base + 2);
                    }
                }

                ui.painter().add(area);

                rect
            })
    }

    fn gradient(&self, volume: &VolumeFractionSettings, t: f32) -> Color32 {
        let color = match volume.nodes.as_slice() {
            [] => [0.5; 3],
            [only] => only.material.albedo(),
            nodes => {
                let index = nodes.partition_point(|node| node.position < t);

                if index == 0 {
                    nodes[0].material.albedo()
                } else if index == nodes.len() {
                    nodes[index - 1].material.albedo()
                } else {
                    let (a, b) = (&nodes[index - 1], &nodes[index]);
                    let span = (b.position - a.position).max(f32::EPSILON);
                    let factor = ((t - a.position) / span).clamp(0.0, 1.0);

                    let (a, b) = (a.material.albedo(), b.material.albedo());
                    std::array::from_fn(|channel| a[channel] + (b[channel] - a[channel]) * factor)
                }
            }
        };

        Color32::from_rgb(
            (color[0] * 255.0).round() as u8,
            (color[1] * 255.0).round() as u8,
            (color[2] * 255.0).round() as u8,
        )
    }

    fn pickers(&mut self, ui: &mut Ui, volume: &mut VolumeFractionSettings, histogram_rect: Rect) {
        let handle_size = 10.0;

        let picker_rect = Rect::from_min_size(
            ui.cursor().min,
            Vec2::new(ui.available_width(), handle_size),
        );

        ui.allocate_space(Vec2::new(ui.available_width(), handle_size));

        let mut clicked = None;
        let volume_id = volume.id;

        for (index, picker) in volume.nodes.iter_mut().enumerate() {
            if Self::picker(
                self,
                ui,
                volume_id,
                histogram_rect,
                picker_rect,
                picker,
                index + 1,
            ) {
                clicked = Some(index);
            }
        }

        if let Some(clicked) = clicked {
            for (index, picker) in volume.nodes.iter_mut().enumerate() {
                picker.selected = index == clicked;
            }
        }
    }

    fn picker(
        &mut self,
        ui: &mut Ui,
        volume_id: u64,
        histogram_rect: Rect,
        picker_rect: Rect,
        node: &mut MaterialNode,
        rank: usize,
    ) -> bool {
        let handle_size = 10.0;
        let line_width = 6.0;

        let x = histogram_rect.min.x + node.position * histogram_rect.width();

        let interact_rect = Rect::from_min_max(
            Pos2::new(x - line_width * 0.5, histogram_rect.min.y),
            Pos2::new(x + line_width * 0.5, picker_rect.max.y),
        );

        let id = ui
            .id()
            .with("transfer_picker")
            .with(volume_id)
            .with(node.id);
        let response = ui.interact(interact_rect, id, Sense::click_and_drag());

        if response.dragged() {
            let delta = response.drag_delta().x / histogram_rect.width();
            node.position = (node.position + delta).clamp(0.0, 1.0);
            self.changed = true;
        }

        let x = histogram_rect.min.x + node.position * histogram_rect.width();

        let fill = if node.selected {
            Color32::WHITE
        } else {
            Color32::from_gray(180)
        };

        let stroke = Stroke::new(if node.selected { 1.5_f32 } else { 1.0 }, fill);

        ui.painter().line_segment(
            [
                Pos2::new(x, histogram_rect.min.y),
                Pos2::new(x, picker_rect.min.y),
            ],
            stroke,
        );

        let top = Pos2::new(x, picker_rect.min.y);
        let points = vec![
            top,
            Pos2::new(x - handle_size * 0.5, picker_rect.min.y + handle_size),
            Pos2::new(x + handle_size * 0.5, picker_rect.min.y + handle_size),
        ];

        ui.painter()
            .add(Shape::convex_polygon(points, fill, stroke));

        let centroid = Pos2::new(x, picker_rect.min.y + handle_size * (2.0 / 3.0));

        ui.painter().text(
            centroid,
            Align2::CENTER_CENTER,
            rank.to_string(),
            FontId::monospace(8.0),
            Color32::BLACK,
        );

        if response.hovered() || response.dragged() {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }

        response.clicked() || response.drag_started()
    }

    fn settings(
        &mut self,
        ui: &mut Ui,
        volume_id: u64,
        node: &mut MaterialNode,
        spacing: Vec2,
        rank: usize,
    ) -> bool {
        ui.scope_builder(
            UiBuilder::new().id(Id::new(("transfer_settings", volume_id, node.id))),
            |ui| {
            let mut frame = Frame::group(&ui.style()).outer_margin(0.0);

            if node.selected {
                frame = frame.stroke(Stroke::new(1.0_f32, Color32::WHITE));
            }

            let frame_response = frame.show(ui, |ui| {
                ui.take_available_width();

                let header = CollapsingHeader::new(format!("{rank}"))
                    .id_salt("material")
                    .show(ui, |ui| {
                        Grid::new("grid")
                            .num_columns(2)
                            .spacing(spacing)
                            .show(ui, |ui| {
                                ui.label("Absorption").on_hover_text(
                                    "Volume Absorption per millimeter.\nHow much light is absorbed by the volume.",
                                );
                                ui.hdr_color_edit(&mut node.material.absorption).track(self);
                                ui.end_row();

                                ui.label("Scattering").on_hover_text(
                                    "Volume Scattering per millimeter.\nHow much light is scattered by the volume.",
                                );
                                ui.hdr_color_edit(&mut node.material.scattering).track(self);
                                ui.end_row();

                                ui.label("IOR").on_hover_text("Adjust Index Of Refraction");
                                ui.slider(&mut node.material.ior, 1.0..=5.0).track(self);
                                ui.end_row();
                            });
                    });

                ui.inline(&header.header_response, |ui| {
                    let albedo = node.material.albedo();
                    let swatch_color = Color32::from_rgb(
                        (albedo[0] * 255.0).round() as u8,
                        (albedo[1] * 255.0).round() as u8,
                        (albedo[2] * 255.0).round() as u8,
                    );

                    // Fixed width (fitting the widest option) so the row doesn't
                    // resize as different presets - with different label lengths -
                    // get selected.
                    let font_id = TextStyle::Button.resolve(ui.style());
                    let text_color = ui.visuals().text_color();

                    let label_width = MaterialPreset::iter()
                        .map(|preset| {
                            ui.painter()
                                .layout_no_wrap(preset.label().to_string(), font_id.clone(), text_color)
                                .size()
                                .x
                        })
                        .fold(0.0_f32, f32::max);

                    let combo_width =
                        label_width + ui.spacing().button_padding.x * 4.0 + ui.spacing().icon_width;

                    ComboBox::from_id_salt("preset")
                        .selected_text(node.preset.label())
                        .width(combo_width)
                        .show_ui(ui, |ui| {
                            for preset in MaterialPreset::iter() {
                                let changed = ui
                                    .selectable_value(&mut node.preset, preset, preset.label())
                                    .track(self)
                                    .changed();

                                if changed {
                                    if let Some(material) = preset.material() {
                                        node.material = material
                                    }
                                }
                            }
                        })
                        .response
                        .on_hover_text("Material preset for this transfer function stop.");

                    let size = Vec2::splat(ui.spacing().interact_size.y);
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, swatch_color);
                });
            });

            // A preset only stays accurate as long as the material matches its
            // canonical values; any drift downgrades it to Custom. Checking this
            // directly (instead of reacting to a "did some widget report changed
            // this frame" flag) is immune to widgets reporting a change for reasons
            // other than a deliberate edit - e.g. `egui::Slider` rounds and writes
            // back its bound value to its `fixed_decimals` count on every render.
            // `approx_eq` tolerates that rounding (the IOR slider only displays 2
            // decimals) so it doesn't itself look like an edit.
            if node
                .preset
                .material()
                .is_some_and(|material| !material.approx_eq(&node.material))
            {
                node.preset = MaterialPreset::Custom;
            }

            frame_response.response.contains_pointer() && ui.input(|i| i.pointer.primary_clicked())
            },
        )
        .inner
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
