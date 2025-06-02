pub mod scalar;
pub mod tractogram;

use tractogram::Tractogram;

pub struct Asset {
    pub tractogram: Option<Tractogram>,
}
