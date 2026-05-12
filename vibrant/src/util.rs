use egui::Response;

pub trait Tracked {
    fn track(&mut self);
}

pub trait ResponseExtentions {
    fn track<T: Tracked>(self, tracked: &mut T) -> Self;
}

impl ResponseExtentions for Response {
    fn track<T: Tracked>(self, tracked: &mut T) -> Self {
        let changed = self.changed();

        if changed {
            tracked.track();
        }

        self
    }
}
