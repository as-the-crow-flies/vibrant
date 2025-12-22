pub mod line;
pub mod texture;
pub mod transform;
pub mod volume;

use line::LineBuffer;
use transform::TransformBuffer;

use crate::asset::volume::VolumeBuffer;

#[derive(Default)]
pub struct Asset {
    pub line: Option<LineBuffer>,
    pub volumes: Vec<VolumeBuffer>,
    pub transform: Option<TransformBuffer>,
}
