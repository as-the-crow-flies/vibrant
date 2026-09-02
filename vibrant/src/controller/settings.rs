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
    pub volume: u32,
    pub index_buffer_size: u32,
    pub radius: f32,
    pub lighting: f32,
    pub ambient_light: f32,
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
    /// Present to an HDR (scRGB) swapchain instead of ACES-to-SDR. Ignored when
    /// [`Surface::hdr_supported`](crate::surface::Surface::hdr_supported) is false.
    pub hdr: bool,
    /// Desired HDR peak, as a multiple of SDR white. Clamped to what the display
    /// actually reports it can drive at present time.
    pub hdr_headroom: f32,
    /// Accumulate successive samples into a persistent buffer while the scene is
    /// unchanged (progressive refinement + sub-pixel AA). When off, every frame
    /// renders from scratch like a game loop and the app never idles.
    pub accumulate: bool,
    /// Hard cap on accumulated samples; accumulation stops here even if the
    /// noise metric never drops below `noise_threshold`.
    pub max_samples: u32,
    /// Relative-residual threshold below which the image counts as converged and
    /// the render loop goes idle. Larger = stop sooner (noisier).
    pub noise_threshold: f32,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            width: 1920,
            height: 1080,
            volume: 256,
            index_buffer_size: 64,
            radius: 0.3,
            lighting: 0.85,
            ambient_light: 1.0,
            direct_light: 1.0,
            tangent_color: 1.0,
            shadows: 0.0,
            alpha: 1.0,
            level: 0.0,
            smoothing: 1.0,
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
            display: LineDisplayMode::Geometry,
            voxelization: LineVoxelizationMode::Tube,
            hdr: false,
            hdr_headroom: 1.0,
            accumulate: true,
            max_samples: 512,
            noise_threshold: 1.0e-3,
        }
    }
}
