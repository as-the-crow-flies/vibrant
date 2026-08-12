pub mod def;

use strum::EnumIter;

use crate::asset::material::def::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub absorption: [f32; 3],
    pub scattering: [f32; 3],
    pub ior: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            absorption: [1.0, 1.0, 1.0],
            scattering: [1.0, 1.0, 1.0],
            ior: 1.0,
        }
    }
}

impl Material {
    pub fn albedo(&self) -> [f32; 3] {
        std::array::from_fn(|channel| {
            let total = self.absorption[channel] + self.scattering[channel];

            if total > 0.0 {
                self.scattering[channel] / total
            } else {
                0.0
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum MaterialPreset {
    Custom,
    Air,
    GreyMatter,
    WhiteMatter,
    Brainstem,
    Optic,
    CSF,
    BloodArterial,
    BloodVenous,
    DuraMater,
    Skull,
    Scalp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialNode {
    pub id: u64,
    pub selected: bool,
    pub position: f32,
    pub preset: MaterialPreset,
    pub material: Material,
}

impl MaterialPreset {
    pub fn label(self) -> &'static str {
        match self {
            MaterialPreset::Custom => "Custom",
            MaterialPreset::Air => "Air",
            MaterialPreset::WhiteMatter => "White Matter",
            MaterialPreset::GreyMatter => "Grey Matter",
            MaterialPreset::Brainstem => "Brainstem",
            MaterialPreset::Optic => "Optic Nerve",
            MaterialPreset::CSF => "CSF",
            MaterialPreset::BloodArterial => "Blood Arterial",
            MaterialPreset::BloodVenous => "Blood Venous",
            MaterialPreset::DuraMater => "Dura Mater",
            MaterialPreset::Skull => "Skull",
            MaterialPreset::Scalp => "Scalp",
        }
    }

    pub fn material(self) -> Option<Material> {
        match self {
            MaterialPreset::Custom => None,
            MaterialPreset::Air => Some(AIR),
            MaterialPreset::GreyMatter => Some(GRAY_MATTER),
            MaterialPreset::WhiteMatter => Some(WHITE_MATTER),
            MaterialPreset::Brainstem => Some(PONS_BRAINSTEM),
            MaterialPreset::Optic => Some(OPTIC_NERVE),
            MaterialPreset::CSF => Some(CSF),
            MaterialPreset::BloodArterial => Some(BLOOD_ARTERIAL),
            MaterialPreset::BloodVenous => Some(BLOOD_VENOUS),
            MaterialPreset::DuraMater => Some(DURA_MATER),
            MaterialPreset::Skull => Some(SKULL),
            MaterialPreset::Scalp => Some(SCALP),
        }
    }
}
