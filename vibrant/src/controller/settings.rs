pub struct Settings {
    pub ambient_occlusion_samples: u32,
    pub streamline_radius: f32,
    pub direct_light: f32,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            ambient_occlusion_samples: 20,
            streamline_radius: 0.1,
            direct_light: 0.8,
        }
    }
}
