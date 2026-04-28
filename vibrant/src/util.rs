use egui::Response;

pub trait Tracked {
    fn track(&mut self);
}

pub trait ResponseExtentions {
    fn track<T: Tracked>(&self, tracked: &mut T) -> bool;
}

impl ResponseExtentions for Response {
    fn track<T: Tracked>(&self, tracked: &mut T) -> bool {
        let changed = self.changed();

        if changed {
            tracked.track();
        }

        changed
    }
}
