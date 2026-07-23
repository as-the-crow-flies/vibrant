use crate::asset::radiance::{gaussian::GaussianRadianceBuffer, linear::LinearRadianceBuffer};

pub mod gaussian;
pub mod linear;

pub enum RadianceBuffer {
    None,
    Linear(LinearRadianceBuffer),
    Gaussian(GaussianRadianceBuffer),
}
