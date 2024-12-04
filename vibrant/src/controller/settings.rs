#[derive(Debug, PartialEq, Eq)]
pub enum DensitySetting {
    Add,
    Or,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GeometrySetting {
    LineHardware,
    LineSoftware,
    Tube,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShadingSetting {
    Tracing,
    Simple,
    Density,
    Occlusion,
    GBuffer,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CullingSetting {
    On,
    Off,
}

pub struct Settings {
    pub ambient_occlusion_samples: u32,
    pub streamline_radius: f32,
    pub direct_light: f32,
    pub gradient_factor: f32,
    pub opacity_factor: f32,
    pub step_size: f32,
    pub grad_size: f32,
    pub min_value: f32,
    pub shading_level: f32,
    pub cull_level: f32,
    pub geometry: GeometrySetting,
    pub shading: ShadingSetting,
    pub culling: CullingSetting,
    pub density: DensitySetting,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            ambient_occlusion_samples: 20,
            streamline_radius: 0.15,
            direct_light: 0.25,
            gradient_factor: 1.0,
            opacity_factor: 0.0035,
            step_size: 0.1,
            grad_size: 1.0,
            min_value: 0.0,
            shading_level: 1.0,
            cull_level: 1.0,
            geometry: GeometrySetting::Tube,
            shading: ShadingSetting::Tracing,
            culling: CullingSetting::On,
            density: DensitySetting::Add,
        }
    }
}
