pub mod density;
pub mod filter;
pub mod scalar;
pub mod tractogram;

use density::Density;
use tractogram::Tractogram;

pub struct Asset {
    pub tractogram: Option<Tractogram>,
    pub density: Density,
}
