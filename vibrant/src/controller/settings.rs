#[derive(Debug, PartialEq, Eq)]
pub enum GeometrySetting {
    Line,
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

#[derive(Debug)]
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
            streamline_radius: 0.2,
            direct_light: 0.72,
            culling_threshold: 4.0,
            alpha: 0.01,
            geometry: GeometrySetting::Tube,
            shading: ShadingSetting::Tracing,
        }
    }
}
