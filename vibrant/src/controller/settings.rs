#[derive(Debug, PartialEq, Eq)]
pub enum GeometrySetting {
    LineHardware,
    Tube,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShadingSetting {
    Simple,
    GBuffer,
    Tracing,
}

pub struct Settings {
    pub streamline_radius: f32,
    pub direct_light: f32,
    pub geometry: GeometrySetting,
    pub shading: ShadingSetting,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            streamline_radius: 0.15,
            direct_light: 0.4,
            geometry: GeometrySetting::LineHardware,
            shading: ShadingSetting::Tracing,
        }
    }
}
