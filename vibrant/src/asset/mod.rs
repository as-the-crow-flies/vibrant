pub mod line;
pub mod radiance;
pub mod segmentation;
pub mod texture;
pub mod transform;
pub mod volume;

use line::LineBuffer;
use segmentation::VolumeSegmenationBuffer;
use transform::TransformBuffer;
use volume::PhysicalVolume;

use crate::asset::radiance::RadianceVolume;

#[derive(Default)]
pub struct Asset {
    pub transform: Option<TransformBuffer>,
    pub line: Option<LineBuffer>,
    pub segmentations: Vec<VolumeSegmenationBuffer>,

    pub physical_volume: Option<PhysicalVolume>,
    pub radiance: Option<RadianceVolume>,
}
