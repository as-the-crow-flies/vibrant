#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SelectionVolume {
    #[default]
    None,
    Box,
    Sphere,
}

#[derive(Debug, Clone)]
pub struct SelectionVolumeEntry {
    pub shape: SelectionVolume,
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub offset_z: f32,
    pub negate: bool,
    pub highlight: bool,
}

impl Default for SelectionVolumeEntry {
    fn default() -> Self {
        Self {
            shape: SelectionVolume::Sphere,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            offset_z: 0.0,
            negate: false,
            highlight: false,
        }
    }
}
