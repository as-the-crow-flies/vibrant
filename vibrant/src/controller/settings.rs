#[derive(Debug, PartialEq, Eq)]
pub enum GeometrySetting {
    Line,
    Tube,
    Transparency,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShadingSetting {
    Render,
    Density,
}

#[derive(Debug)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub density: u32,
    pub occlusion: u32,
    pub memory: u32,
    pub streamline_radius: f32,
    pub direct_light: f32,
    pub culling_threshold: f32,
    pub alpha: f32,
    pub level: f32,
    pub layer: u32,
    pub skip: u32,
    pub quality: bool,
    pub smoothing: f32,
    pub shading: ShadingSetting,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            width: 1920,
            height: 1080,
            density: 256,
            occlusion: 128,
            memory: 256,
            streamline_radius: 0.2,
            direct_light: 0.25,
            culling_threshold: 4.0,
            alpha: 1.0,
            level: 0.0,
            layer: 0,
            skip: 1,
            quality: false,
            smoothing: 0.5,
            shading: ShadingSetting::Render,
        }
    }
}
