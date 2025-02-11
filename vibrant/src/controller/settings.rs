#[derive(Debug, PartialEq, Eq)]
pub enum GeometrySetting {
    LineHardware,
    Tube,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShadingSetting {
    Simple,
    GBuffer,
    Culling,
    Density,
    Tracing,
}

pub struct Settings {
    pub streamline_radius: f32,
    pub direct_light: f32,
    pub culling_threshold: f32,
    pub alpha: f32,
    pub geometry: GeometrySetting,
    pub shading: ShadingSetting,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            streamline_radius: 0.25,
            direct_light: 0.0,
            culling_threshold: 1.0,
            alpha: 0.01,
            geometry: GeometrySetting::LineHardware,
            shading: ShadingSetting::Tracing,
        }
    }
}
