pub struct Settings {
    pub streamline_radius: f32,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            streamline_radius: 0.1,
        }
    }
}
