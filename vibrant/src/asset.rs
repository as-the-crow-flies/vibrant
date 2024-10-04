pub mod density;
pub mod tractogram;

pub use tractogram::*;

#[derive(Default)]
pub struct Asset {
    pub tractogram: Option<Tractogram>,
}
