pub mod density;
pub mod tractogram;

use density::Density;
pub use tractogram::*;

pub struct Asset {
    pub tractogram: Option<Tractogram>,
    pub density: Density,
}
