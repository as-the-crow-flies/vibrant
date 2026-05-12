use std::ops::{RangeInclusive, Sub};

use egui::{emath::Numeric, Frame, Response, Slider, Ui};

pub trait UIComponents {
    fn toggle(&mut self, icon: &str, tooltip: &str, selected: &mut bool) -> Response;
    fn toggle_visible(&mut self, selected: &mut bool) -> Response;
    fn toggle_inverted(&mut self, selected: &mut bool) -> Response;
    fn delete(&mut self) -> Response;
    fn frame(&mut self, contents: impl FnOnce(&mut Ui));
    fn slider<'a, Num>(&mut self, value: &'a mut Num, range: RangeInclusive<Num>) -> Response
    where
        Num: Numeric;
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
            .show(self, contents);
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
}
