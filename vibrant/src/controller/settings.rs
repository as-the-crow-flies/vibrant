// Anti-aliasing mode selector.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum AntiAliasingMode {
    // No anti-aliasing applied.
    #[default]
    Off,
    // Super-Sample Anti-Aliasing (render at higher resolution, downsample)
    SSAA,
    // Subpixel Morphological Anti-Aliasing
    SMAA,
    // Temporal Anti-Aliasing
    TAA,
    // Adaptive: auto-switch based on camera motion (Fast→SMAA, Slow→TAA, Still→SSAA)
    Adaptive,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecordingMode {
    #[default]
    Performance,
    Quality,
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
    // Current anti-aliasing mode.
    pub aa_mode: AntiAliasingMode,
    // SMAA edge detection threshold
    pub smaa_threshold: f32,
    // SMAA max search steps
    pub smaa_max_search_steps: u32,
    // TAA weight of the current frame in the temporal blend
    pub taa_blend_factor: f32,
    // TAA variance clipping sigma, controls how aggressively stale history is rejected
    pub taa_clamp_sigma: f32,
    // resolved AA mode
    pub effective_aa_mode: AntiAliasingMode,
    pub recording:  bool,
    pub record_path: &'static str,
    pub recording_delay: u32,
    pub recording_fps: u32,
    pub recording_mode: RecordingMode,
    pub foveated: bool,
    pub foveated_focus_radius: f32,
    pub foveated_peripheral_scale: f32,
    pub foveated_focus_scale: f32,
    pub foveated_blend_width: f32,
    pub foveated_mouse_x: f32,
    pub foveated_mouse_y: f32,
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
            bloom_enabled: false,
            bloom_threshold: 0.5,
            bloom_soft_knee: 1.0,
            bloom_intensity: 3.0,
            hdr_paper_white_nits: 200.0,
            hdr_peak_nits: 1000.0,
            recording: false,
            record_path: "/Users/user/Desktop/TUe/Visual Computing Project/vibrant/recordings/recording.mp4",
            display: LineDisplayMode::Geometry,
            voxelization: LineVoxelizationMode::Tube,
            recording_delay: 0,
            recording_fps: 30,
            aa_mode: AntiAliasingMode::Off,
            smaa_threshold: 0.1,
            smaa_max_search_steps: 16,
            taa_blend_factor: 0.15,
            taa_clamp_sigma: 1.0,
            effective_aa_mode: AntiAliasingMode::Off,
            recording_mode: RecordingMode::Performance,
            foveated: false,
            foveated_focus_radius: 0.15,
            foveated_peripheral_scale: 0.5,
            foveated_focus_scale: 1.5,
            foveated_blend_width: 0.05,
            foveated_mouse_x: 0.0,
            foveated_mouse_y: 0.0,
        }
    }

    pub fn update_render_size(&mut self) {
        self.render_width = (self.width as f32 * self.render_scale).round().max(1.0) as u32;
        self.render_height = (self.height as f32 * self.render_scale).round().max(1.0) as u32;
    }

    pub fn peripheral_width(&self) -> u32 {
        (self.render_width as f32 * self.foveated_peripheral_scale)
            .round()
            .max(1.0) as u32
    }

    pub fn peripheral_height(&self) -> u32 {
        (self.render_height as f32 * self.foveated_peripheral_scale)
            .round()
            .max(1.0) as u32
    }

    /// Focus texture width — derived from screen coverage × scale factor.
    /// NDC radius r covers r × render_width pixels on screen.
    pub fn focus_width(&self) -> u32 {
        (self.foveated_focus_radius * self.render_width as f32 * self.foveated_focus_scale)
            .round()
            .max(1.0) as u32
    }

    pub fn focus_height(&self) -> u32 {
        (self.foveated_focus_radius * self.render_height as f32 * self.foveated_focus_scale)
            .round()
            .max(1.0) as u32
    }
}
