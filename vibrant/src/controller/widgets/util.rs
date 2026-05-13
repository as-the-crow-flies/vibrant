use egui::{Align, Layout, Response};

pub trait UiResponseExtensions {
    fn help(self, heading: &str, body: &str) -> Response;
}

impl UiResponseExtensions for Response {
    fn help(self, heading: &str, body: &str) -> Response {
        self.on_hover_ui(|ui| {
            ui.with_layout(
                Layout::top_down(Align::LEFT).with_cross_justify(true),
                |ui| {
                    ui.heading(heading);
                    ui.label(body);
                },
            );
        })
    }
}
