pub mod density;
pub mod tractogram;
pub mod volume;

pub use density::Density;
pub use tractogram::*;
pub use volume::Volume;

pub struct Asset {
    pub tractogram: Option<Tractogram>,
    pub volume: Option<Volume>,
    pub density: Density,
}
