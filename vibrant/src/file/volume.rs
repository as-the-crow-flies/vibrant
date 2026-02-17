use std::{f32::consts::PI, io::Cursor};

use flate2::read::GzDecoder;
use glam::{Mat4, Vec3};
use nifti::{InMemNiftiObject, NiftiObject, NiftiType};

use crate::file::File;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeType {
    Uint8,
    Uint16,
    Uint32,
    Int8,
    Int16,
    Int32,
    Float32,
}

impl VolumeType {
    pub fn is_float(&self) -> bool {
        *self == VolumeType::Float32
    }

    pub fn is_integer(&self) -> bool {
        !self.is_float()
    }
}

pub struct VolumeFile {
    name: String,
    ty: VolumeType,
    transform: Mat4,
    data: Vec<u8>,
    dim: Vec<u16>,
}

impl VolumeFile {
    pub fn from_nifti(file: &File) -> Self {
        let obj = InMemNiftiObject::from_reader(GzDecoder::new(Cursor::new(&file.data)))
            .expect("Nifti should contain volume data");

        let dim = obj
            .header()
            .dim()
            .expect("Nifti should contain valid dimensionality");

        let voxel_to_texture = Mat4::from_scale(Vec3::new(
            1.0 / dim[0] as f32,
            1.0 / dim[1] as f32,
            1.0 / dim[2] as f32,
        ));

        let mm_to_voxel = Mat4::from_cols_array_2d(&[
            obj.header().srow_x,
            obj.header().srow_y,
            obj.header().srow_z,
            [0.0, 0.0, 0.0, 1.0],
        ])
        .transpose()
        .inverse();

        let rotation = Mat4::from_rotation_x(PI / 2.0);

        let transform = voxel_to_texture * mm_to_voxel * rotation;

        let ty = match obj.header().data_type().expect("Invalid Nifti data type") {
            NiftiType::Uint8 => VolumeType::Uint8,
            NiftiType::Uint16 => VolumeType::Uint16,
            NiftiType::Uint32 => VolumeType::Uint32,
            NiftiType::Int8 => VolumeType::Int8,
            NiftiType::Int16 => VolumeType::Int16,
            NiftiType::Int32 => VolumeType::Int32,
            NiftiType::Float32 => VolumeType::Float32,
            _ => panic!("Unsupported Nifti data type"),
        };

        let dim = obj
            .header()
            .dim()
            .expect("Invalid Nifti dimension")
            .to_vec();

        Self {
            data: obj.into_volume().into_raw_data(),
            name: file.name.replace(".nii", "").replace(".gz", "").to_owned(),
            transform,
            ty,
            dim,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn transform(&self) -> Mat4 {
        self.transform
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn ty(&self) -> VolumeType {
        self.ty
    }

    pub fn dim(&self) -> &[u16] {
        &self.dim
    }
}
