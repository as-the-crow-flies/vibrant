use egui::{
    Align2, CollapsingHeader, Color32, ComboBox, FontId, Frame, Grid, Id, Mesh, Pos2, Rect, Sense,
    Shape, Stroke, TextStyle, Ui, UiBuilder, Vec2,
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

struct SettingsResponse {
    selected: bool,
    deleted: bool,
}

struct HistogramResponse {
    rect: Rect,
    add_at: Option<f32>,
}

struct PickerResponse {
    selected: bool,
    deleted: bool,
}

impl Tracked for TransferFunctionEditor {
    fn track(&mut self) {
        self.changed = true;
    }
}

impl Default for TransferFunctionEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferFunctionEditor {
    const HANDLE_SIZE: f32 = 10.0;

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

        volume
            .nodes
            .sort_by(|a, b| a.position.total_cmp(&b.position));

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            let histogram = self.histogram(ui, volume);
            let picker_deleted = self.pickers(ui, volume, histogram.rect);

            if let Some(index) = picker_deleted {
                volume.nodes.remove(index);

                if let Some(last) = volume.nodes.last_mut() {
                    last.selected = true;
                }

                self.changed = true;
            } else if let Some(position) = histogram.add_at {
                if volume.nodes.len() < MaterialNodeBuffer::MAX_NODES {
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

                    self.changed = true;
                }
            }
        });

        let mut clicked = None;
        let mut deleted = None;
        let volume_id = volume.id;

        for (index, node) in volume.nodes.iter_mut().enumerate() {
            let settings = self.settings(ui, volume_id, node, spacing, index + 1);

            if settings.selected {
                clicked = Some(index);
            }

            if settings.deleted {
                deleted = Some(index);
            }
        }

        if let Some(clicked) = clicked {
            for (index, node) in volume.nodes.iter_mut().enumerate() {
                node.selected = index == clicked;
            }
        }

        if let Some(index) = deleted {
            volume.nodes.remove(index);

            if let Some(last) = volume.nodes.last_mut() {
                last.selected = true;
            }

            self.changed = true;
        }
    }

    fn histogram(&self, ui: &mut Ui, volume: &mut VolumeFractionSettings) -> HistogramResponse {
        Frame::canvas(ui.style())
            .fill(ui.visuals().extreme_bg_color)
            .show(ui, |ui| {
                let size = Vec2::new(ui.available_width(), 4.0 * ui.spacing().interact_size.y);
                let rect = Rect::from_min_size(ui.cursor().min, size);

                let id = ui.id().with("transfer_histogram").with(volume.id);
                let response = ui
                    .interact(rect, id, Sense::click())
                    .on_hover_text("Double-click to add/remove a color stop.");

                let add_at = (response.double_clicked())
                    .then(|| response.interact_pointer_pos())
                    .flatten()
                    .map(|pos| ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0));

                if response.hovered() {
                    ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::Crosshair);
                }

                ui.allocate_space(size);
                // Intersect, don't replace: replacing lets the mesh paint over
                // the top panel when scrolled out of view and keeps it at full
                // height during the Volumes collapse animation.
                ui.set_clip_rect(rect.intersect(ui.clip_rect()));

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

                HistogramResponse { rect, add_at }
            })
            .inner
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

        Self::albedo_color32(color)
    }

    fn albedo_color32(albedo: [f32; 3]) -> Color32 {
        Color32::from_rgb(
            (albedo[0] * 255.0).round() as u8,
            (albedo[1] * 255.0).round() as u8,
            (albedo[2] * 255.0).round() as u8,
        )
    }

    fn contrasting_text_color(bg: Color32) -> Color32 {
        let luminance = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;

        if luminance > 140.0 {
            Color32::BLACK
        } else {
            Color32::WHITE
        }
    }

    fn pickers(
        &mut self,
        ui: &mut Ui,
        volume: &mut VolumeFractionSettings,
        histogram_rect: Rect,
    ) -> Option<usize> {
        let size = Vec2::new(ui.available_width(), Self::HANDLE_SIZE);
        let picker_rect = Rect::from_min_size(ui.cursor().min, size);

        ui.allocate_space(size);

        let mut clicked = None;
        let mut deleted = None;
        let volume_id = volume.id;

        for (index, picker) in volume.nodes.iter_mut().enumerate() {
            let response = self.picker(
                ui,
                volume_id,
                histogram_rect,
                picker_rect,
                picker,
                index + 1,
            );

            if response.selected {
                clicked = Some(index);
            }

            if response.deleted {
                deleted = Some(index);
            }
        }

        if let Some(clicked) = clicked {
            for (index, picker) in volume.nodes.iter_mut().enumerate() {
                picker.selected = index == clicked;
            }
        }

        deleted
    }

    fn picker(
        &mut self,
        ui: &mut Ui,
        volume_id: u64,
        histogram_rect: Rect,
        picker_rect: Rect,
        node: &mut MaterialNode,
        rank: usize,
    ) -> PickerResponse {
        let handle_size = Self::HANDLE_SIZE;
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

        PickerResponse {
            selected: response.clicked() || response.drag_started(),
            deleted: response.double_clicked(),
        }
    }

    fn settings(
        &mut self,
        ui: &mut Ui,
        volume_id: u64,
        node: &mut MaterialNode,
        spacing: Vec2,
        rank: usize,
    ) -> SettingsResponse {
        let response = ui.scope_builder(
            UiBuilder::new().id(Id::new(("transfer_settings", volume_id, node.id))),
            |ui| {
            let mut frame = Frame::group(ui.style()).outer_margin(0.0);

            if node.selected {
                frame = frame.stroke(Stroke::new(1.0_f32, Color32::WHITE));
            }

            let mut deleted = false;

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
                    if ui.delete().track(self).clicked() {
                        deleted = true;
                    }

                    let swatch_width =
                        ui.spacing().interact_size.x + ui.spacing().item_spacing.x;
                    let reserved_left =
                        ui.spacing().indent + 4.0 * ui.spacing().button_padding.x + swatch_width;
                    let combo_width = (ui.available_width() - reserved_left).max(0.0);

                    self.preset_combo(ui, node, combo_width);

                    let density = node
                        .material
                        .absorption
                        .iter()
                        .zip(&node.material.scattering)
                        .map(|(a, s)| a + s)
                        .fold(0.0_f32, f32::max)
                        .clamp(0.0, 1.0);
                    let mut color = node.material.albedo().map(|channel| channel * density);

                    if ui
                        .color_edit_button_rgb(&mut color)
                        .on_hover_text("Volume color")
                        .track(self)
                        .changed()
                    {
                        let density = color.iter().cloned().fold(0.0_f32, f32::max);

                        node.material.scattering = color;
                        node.material.absorption = color.map(|channel| density - channel);
                    }
                });
            });

            if node
                .preset
                .material()
                .is_some_and(|material| !material.approx_eq(&node.material))
            {
                node.preset = MaterialPreset::Custom;
            }

            SettingsResponse {
                selected: frame_response.response.contains_pointer()
                    && ui.input(|i| i.pointer.primary_clicked()),
                deleted,
            }
            },
        );

        response.inner
    }

    fn preset_combo(&mut self, ui: &mut Ui, node: &mut MaterialNode, combo_width: f32) {
        let bg = Self::albedo_color32(node.material.albedo());
        let fg = Self::contrasting_text_color(bg);

        ui.scope(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.weak_bg_fill = bg;
            widgets.inactive.fg_stroke.color = fg;
            widgets.hovered.weak_bg_fill = bg;
            widgets.hovered.fg_stroke.color = fg;
            widgets.open.weak_bg_fill = bg;
            widgets.open.fg_stroke.color = fg;

            ComboBox::from_id_salt("preset")
                .selected_text(node.preset.label())
                .width(combo_width)
                .show_ui(ui, |ui| {
                    for preset in MaterialPreset::iter().skip(1) {
                        self.preset_row(ui, node, preset);
                    }
                })
                .response
                .on_hover_text("Material preset for this transfer function stop.");
        });
    }

    fn preset_row(&mut self, ui: &mut Ui, node: &mut MaterialNode, preset: MaterialPreset) {
        let selected = node.preset == preset;

        let albedo = preset
            .material()
            .unwrap_or_else(|| node.material.clone())
            .albedo();
        let row_bg = Self::albedo_color32(albedo);
        let row_fg = Self::contrasting_text_color(row_bg);

        let padding = ui.spacing().button_padding;
        let font_id = TextStyle::Button.resolve(ui.style());
        let galley = ui
            .painter()
            .layout_no_wrap(preset.label().to_string(), font_id, row_fg);

        let row_size = Vec2::new(ui.available_width(), galley.size().y + 2.0 * padding.y);
        let (row_rect, response) = ui.allocate_exact_size(row_size, Sense::click());

        if response.hovered() {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::PointingHand);
        }

        ui.painter().rect_filled(row_rect, 2.0, row_bg);

        if selected || response.hovered() {
            ui.painter().rect_stroke(
                row_rect,
                2.0,
                Stroke::new(1.5_f32, Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        }

        ui.painter().galley(row_rect.min + padding, galley, row_fg);

        if response.clicked() && !selected {
            node.preset = preset;
            self.track();

            if let Some(material) = preset.material() {
                node.material = material
            }
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
