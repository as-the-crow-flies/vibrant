#[derive(Debug, PartialEq, Eq)]
pub enum GeometrySetting {
    LineHardware,
    Tube,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShadingSetting {
    Simple,
    GBuffer,
}

pub struct Settings {
    pub streamline_radius: f32,
    pub geometry: GeometrySetting,
    pub shading: ShadingSetting,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            streamline_radius: 0.15,
            geometry: GeometrySetting::LineHardware,
            shading: ShadingSetting::Simple,
        }
    }
}
