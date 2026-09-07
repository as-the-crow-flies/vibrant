pub mod transfer;

use egui::{
    emath::Numeric, pos2, Align, CollapsingHeader, Color32, DragValue, Frame, Id, Layout, Rect,
    Response, Sense, Slider, StrokeKind, TextStyle, Ui, Vec2, Widget, WidgetText,
};
use std::ops::{RangeInclusive, Sub};

pub trait UIComponents {
    fn toggle(&mut self, icon: &str, tooltip: &str, selected: &mut bool) -> Response;
    fn toggle_visible(&mut self, selected: &mut bool) -> Response;
    fn toggle_inverted(&mut self, selected: &mut bool) -> Response;
    fn delete(&mut self) -> Response;
    fn frame(&mut self, contents: impl FnOnce(&mut Ui));

    /// A fixed-size square icon button (e.g. a single emoji glyph). Hand-painted
    /// instead of built from `egui::Button`, whose size is content-driven - a
    /// `min_size` floor can't shrink a glyph that already renders wider than it,
    /// so different icons (e.g. "➕" vs "🗑") end up different widths.
    fn icon_button(&mut self, icon: &str, enabled: bool, selected: bool) -> Response;
    fn slider<Num>(&mut self, value: &mut Num, range: RangeInclusive<Num>) -> Response
    where
        Num: Numeric;

    /// Per-channel color editor for values that can exceed 1.0.
    fn hdr_color_edit(&mut self, value: &mut [f32; 3]) -> Response;

    fn collapse(
        &mut self,
        header: impl Into<WidgetText>,
        open: bool,
        body: impl FnOnce(&mut Ui),
    ) -> Response;

    fn inline(&mut self, response: &Response, contents: impl FnOnce(&mut Ui));

    fn is_new(&mut self, name: &str) -> bool;
}

impl UIComponents for Ui {
    fn toggle(&mut self, icon: &str, tooltip: &str, selected: &mut bool) -> Response {
        let mut response = self.icon_button(icon, true, *selected);

        if response.clicked() {
            *selected = !*selected;
            response.mark_changed();
        }

        response.on_hover_text(tooltip)
    }

    fn toggle_visible(&mut self, selected: &mut bool) -> Response {
        self.toggle("👁", "Show or hide this in the 3D view.", selected)
    }

    fn toggle_inverted(&mut self, selected: &mut bool) -> Response {
        self.toggle(
            "🌗",
            "Invert this mask: swap the region it keeps for the region it removes.",
            selected,
        )
    }

    fn delete(&mut self) -> Response {
        self.icon_button("🗑", true, false)
            .on_hover_text("Remove this item.")
    }

    fn icon_button(&mut self, icon: &str, enabled: bool, selected: bool) -> Response {
        let size = Vec2::splat(self.spacing().interact_size.y);
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = self.allocate_exact_size(size, sense);

        if self.is_rect_visible(rect) {
            let visuals = self.style().interact_selectable(&response, selected);

            self.painter().rect(
                rect.expand(visuals.expansion),
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                StrokeKind::Inside,
            );

            // `Painter::text`'s CENTER_CENTER anchors on the galley's line-height
            // box, not the glyph's visible ink - fine for latin text, but many
            // emoji glyphs (e.g. "🗑", "👁") sit asymmetrically within that box, so
            // it visibly off-centers them. Center on `mesh_bounds` (the glyph's
            // actual rendered bounds) instead.
            let font_id = TextStyle::Button.resolve(self.style());
            let galley =
                self.painter()
                    .layout_no_wrap(icon.to_string(), font_id, visuals.text_color());

            let paint_pos = if galley.mesh_bounds.is_positive() {
                rect.center() - (galley.mesh_bounds.center() - galley.rect.min)
            } else {
                rect.center() - galley.size() / 2.0
            };

            self.painter()
                .galley(paint_pos, galley, visuals.text_color());
        }

        response
    }

    fn frame(&mut self, contents: impl FnOnce(&mut Ui)) {
        Frame::group(self.style())
            .fill(self.visuals().faint_bg_color)
            .corner_radius(6.0)
            .inner_margin(6.0)
            .show(self, |ui| {
                ui.take_available_width();
                contents(ui);
            });
    }

    fn slider<Num: Numeric>(&mut self, value: &mut Num, range: RangeInclusive<Num>) -> Response {
        self.spacing_mut().slider_width = self
            .available_width()
            .sub(self.spacing().interact_size.x)
            .sub(self.spacing().button_padding.x * 2.0)
            .max(0.0);
        self.add(Slider::new(value, range).fixed_decimals(2))
    }

    fn collapse(
        &mut self,
        header: impl Into<WidgetText>,
        open: bool,
        body: impl FnOnce(&mut Ui),
    ) -> Response {
        CollapsingHeader::new(header)
            .open(match open {
                true => Some(true),
                false => None,
            })
            .show(self, body)
            .header_response
    }

    fn inline(&mut self, response: &Response, contents: impl FnOnce(&mut Ui)) {
        self.place(
            Rect {
                min: response.rect.min,
                max: pos2(
                    response.rect.min.x + self.available_width(),
                    response.rect.max.y,
                ),
            },
            ClosureWidget(|ui| {
                ui.with_layout(Layout::right_to_left(Align::TOP), contents)
                    .response
            }),
        );
    }

    fn hdr_color_edit(&mut self, value: &mut [f32; 3]) -> Response {
        let scale = value.iter().copied().fold(1.0_f32, f32::max);
        let preview = value.map(|channel| (channel / scale).clamp(0.0, 1.0));

        let color = Color32::from_rgb(
            (preview[0] * 255.0).round() as u8,
            (preview[1] * 255.0).round() as u8,
            (preview[2] * 255.0).round() as u8,
        );

        self.horizontal(|ui| {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::splat(ui.spacing().interact_size.y), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);

            let mut response: Option<Response> = None;

            for (channel, label) in value.iter_mut().zip(["R", "G", "B"]) {
                let channel_response = ui.add(
                    DragValue::new(channel)
                        .range(0.0..=f64::MAX)
                        .speed(0.01)
                        .fixed_decimals(3)
                        .prefix(format!("{label} ")),
                );

                response = Some(match response {
                    Some(response) => response | channel_response,
                    None => channel_response,
                });
            }

            response.unwrap()
        })
        .inner
    }

    fn is_new(&mut self, name: &str) -> bool {
        let id = Id::new(name);
        let is_new = self.data(|data| data.get_temp::<()>(id).is_none());

        if is_new {
            self.data_mut(|d| d.insert_temp(id, ()));
        }

        is_new
    }
}

struct ClosureWidget<T: FnOnce(&mut Ui) -> Response>(T);

impl<T: FnOnce(&mut Ui) -> Response> Widget for ClosureWidget<T> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.0(ui)
    }
}
