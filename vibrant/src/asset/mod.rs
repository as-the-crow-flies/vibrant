pub mod density;
pub mod filter;
pub mod occlusion;
pub mod scalar;
pub mod tractogram;
pub mod volume;

pub use density::Density;
use occlusion::Occlusion;
pub use tractogram::*;
pub use volume::Volume;

pub struct Asset {
    pub tractogram: Option<Tractogram>,
    pub volume: Option<Volume>,
    pub density: Density,
    pub occlusion: Occlusion,
}
