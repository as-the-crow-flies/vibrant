pub mod hdri;
pub mod line;
pub mod radiance;
pub mod segmentation;
pub mod texture;
pub mod transform;
pub mod volume;
pub mod volume_fraction;

use line::LineBuffer;
use segmentation::VolumeSegmenationBuffer;
use transform::TransformBuffer;
use volume::PhysicalVolume;

use crate::asset::{
    hdri::HdriBuffer, radiance::RadianceVolume, volume_fraction::VolumeFractionBuffer,
};

#[derive(Default)]
pub struct Asset {
    pub transform: Option<TransformBuffer>,
    pub line: Option<LineBuffer>,
    pub segmentations: Vec<VolumeSegmenationBuffer>,
    pub volume_fractions: Vec<VolumeFractionBuffer>,
    pub physical_volume: Option<PhysicalVolume>,
    pub radiance: Option<RadianceVolume>,
    pub hdri: Option<HdriBuffer>,
}
