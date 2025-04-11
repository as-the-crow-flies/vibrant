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
    pub level: f32,
    pub skip: u32,
    pub quality: bool,
    pub geometry: GeometrySetting,
    pub shading: ShadingSetting,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            streamline_radius: 0.5,
            direct_light: 0.72,
            culling_threshold: 2.0,
            alpha: 0.01,
            level: 0.0,
            skip: 1,
            quality: true,
            geometry: GeometrySetting::Tube,
            shading: ShadingSetting::Tracing,
        }
    }
}
