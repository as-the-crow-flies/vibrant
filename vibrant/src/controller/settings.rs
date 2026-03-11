#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum LineVoxelizationMode {
    #[default]
    Tube,
    Line,
    Box,
}

impl LineVoxelizationMode {
    pub fn iter() -> Vec<LineVoxelizationMode> {
        vec![
            LineVoxelizationMode::Tube,
            LineVoxelizationMode::Line,
            LineVoxelizationMode::Box,
        ]
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum LineDisplayMode {
    #[default]
    Geometry,
    Volume,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub render_scale: f32,
    pub render_width: u32,
    pub render_height: u32,
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
    pub workgroups: u32,
    pub crop_start: f32,
    pub crop_end: f32,
    pub crop_middle: f32,
    pub crop_x_start: f32,
    pub crop_x_end: f32,
    pub crop_y_start: f32,
    pub crop_y_end: f32,
    pub crop_z_start: f32,
    pub crop_z_end: f32,
    pub display: LineDisplayMode,
    pub voxelization: LineVoxelizationMode,
    pub plane: f32,
    pub bloom_enabled: bool,
    pub bloom_threshold: f32,
    pub bloom_soft_knee: f32,
    pub bloom_intensity: f32,
    pub hdr_paper_white_nits: f32,
    pub hdr_peak_nits: f32,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            width: 1920,
            height: 1080,
            render_scale: 1.0,
            render_width: 1920,
            render_height: 1080,
            volume: 256,
            radius: 0.25,
            lighting: 0.725,
            direct_light: 1.0,
            tangent_color: 1.0,
            shadows: 0.0,
            alpha: 1.0,
            level: 0.0,
            smoothing: 0.67,
            culling: true,
            slice_count: 10,
            crop_start: 0.0,
            crop_end: 1.0,
            crop_middle: 0.0,
            crop_x_start: -0.5,
            crop_x_end: 0.5,
            crop_y_start: -0.5,
            crop_y_end: 0.5,
            crop_z_start: -0.5,
            crop_z_end: 1.0,
            workgroups: 64,
            plane: 0.33,
            bloom_enabled: true,
            bloom_threshold: 0.5,
            bloom_soft_knee: 1.0,
            bloom_intensity: 3.0,
            hdr_paper_white_nits: 200.0,
            hdr_peak_nits: 1000.0,
            display: LineDisplayMode::Geometry,
            voxelization: LineVoxelizationMode::Tube,
        }
    }

    pub fn update_render_size(&mut self) {
        self.render_width = (self.width as f32 * self.render_scale).round().max(1.0) as u32;
        self.render_height = (self.height as f32 * self.render_scale).round().max(1.0) as u32;
    }
}
