#[derive(Debug, PartialEq, Eq)]
pub enum Renderer {
    Baseline,
    Compute,
    Regular,
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
    pub renderer: Renderer,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            ambient_occlusion_samples: 20,
            streamline_radius: 0.1,
            direct_light: 0.8,
            gradient_factor: 1.0,
            opacity_factor: 0.0035,
            step_size: 0.1,
            grad_size: 1.0,
            min_value: 0.0,
            renderer: Renderer::Regular,
        }
    }
}
