use egui::{
    emath::Numeric, pos2, Align, CollapsingHeader, Frame, Id, Layout, Rect, Response, Slider, Ui,
    Widget, WidgetText,
};
use std::ops::{RangeInclusive, Sub};

pub trait UIComponents {
    fn toggle(&mut self, icon: &str, tooltip: &str, selected: &mut bool) -> Response;
    fn toggle_visible(&mut self, selected: &mut bool) -> Response;
    fn toggle_inverted(&mut self, selected: &mut bool) -> Response;
    fn delete(&mut self) -> Response;
    fn frame(&mut self, contents: impl FnOnce(&mut Ui));
    fn slider<'a, Num>(&mut self, value: &'a mut Num, range: RangeInclusive<Num>) -> Response
    where
        Num: Numeric;

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
        self.toggle_value(selected, icon).on_hover_text(tooltip)
    }

    fn toggle_visible(&mut self, selected: &mut bool) -> Response {
        self.toggle("👁", "Show/Hide", selected)
    }

    fn toggle_inverted(&mut self, selected: &mut bool) -> Response {
        self.toggle("🌗", "Invert", selected)
    }

    fn delete(&mut self) -> Response {
        self.button("🗑").on_hover_text("Delete")
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

    fn slider<'a, Num: Numeric>(
        &mut self,
        value: &'a mut Num,
        range: RangeInclusive<Num>,
    ) -> Response {
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

    fn is_new(&mut self, name: &str) -> bool {
        let id = Id::new(&name);
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
