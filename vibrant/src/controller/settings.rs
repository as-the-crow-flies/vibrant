#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LineVoxelizationMode {
    Line,
    Box,
    Tube,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LineRenderMode {
    Rasterization,
    RayTracing,
    Volume,
}

#[derive(Debug)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub volume: u32,
    pub radius: f32,
    pub lighting: f32,
    pub direct_light: f32,
    pub tangent_color: f32,
    pub shadows: f32,
    pub alpha: f32,
    pub level: f32,
    pub smoothing: f32,
    pub culling: bool,
    pub slice_count: u32,
    pub render: LineRenderMode,
    pub voxelization: LineVoxelizationMode,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            width: 1920,
            height: 1080,
            volume: 128,
            radius: 0.2,
            lighting: 1.0,
            direct_light: 0.67,
            tangent_color: 1.0,
            shadows: 0.0,
            alpha: 1.0,
            level: 0.0,
            smoothing: 0.67,
            culling: true,
            slice_count: 10,
            render: LineRenderMode::Rasterization,
            voxelization: LineVoxelizationMode::Tube,
        }
    }
}
