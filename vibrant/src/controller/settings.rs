#[derive(Debug, PartialEq, Eq)]
pub enum Geometry {
    Line,
    Tube,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Shading {
    Tracing,
    Simple,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Culling {
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
    pub debug_level: f32,
    pub cull_level: f32,
    pub geometry: Geometry,
    pub shading: Shading,
    pub culling: Culling,
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
            debug_level: 0.0,
            cull_level: 1.0,
            geometry: Geometry::Tube,
            shading: Shading::Tracing,
            culling: Culling::On,
        }
    }
}
