use std::io::Cursor;

use flate2::read::GzDecoder;
use glam::{Mat4, UVec3, Vec3};
use itertools::Itertools;
use nifti::{
    object::GenericNiftiObject, DataElement, InMemNiftiObject, InMemNiftiVolume, NiftiObject,
    NiftiType,
};
use num::ToPrimitive;

use crate::file::File;

pub struct VolumeFile {
    name: String,
    transform: Mat4,
    data: Vec<f32>,
    dim: Vec<u16>,
}

impl VolumeFile {
    pub fn from_nifti(file: &File) -> Self {
        let nitfi = InMemNiftiObject::from_reader(GzDecoder::new(Cursor::new(&file.data)))
            .expect("Nifti should contain volume data");

        let dim = nitfi
            .header()
            .dim()
            .expect("Nifti should contain valid dimensionality");

        let voxel_to_texture = Mat4::from_scale(Vec3::new(
            1.0 / dim[0] as f32,
            1.0 / dim[1] as f32,
            1.0 / dim[2] as f32,
        ));

        let mm_to_voxel = Mat4::from_cols_array_2d(&[
            nitfi.header().srow_x,
            nitfi.header().srow_y,
            nitfi.header().srow_z,
            [0.0, 0.0, 0.0, 1.0],
        ])
        .transpose()
        .inverse();

        let transform = voxel_to_texture * mm_to_voxel;

        let name = file.name.replace(".nii", "").replace(".gz", "").to_owned();

        let dim = nitfi
            .header()
            .dim()
            .expect("Invalid Nifti dimension")
            .to_vec();

        let ty = nitfi.header().data_type().expect("Invalid Nifti data type");

        let data: Vec<f32> = match ty {
            NiftiType::Uint8 => Self::to_f32::<u8>(nitfi),
            NiftiType::Int16 => Self::to_f32::<i16>(nitfi),
            NiftiType::Int32 => Self::to_f32::<i32>(nitfi),
            NiftiType::Float32 => Self::to_f32::<f32>(nitfi),
            NiftiType::Float64 => Self::to_f32::<f64>(nitfi),
            NiftiType::Int8 => Self::to_f32::<i8>(nitfi),
            NiftiType::Uint16 => Self::to_f32::<u16>(nitfi),
            NiftiType::Uint32 => Self::to_f32::<u32>(nitfi),
            NiftiType::Int64 => Self::to_f32::<i64>(nitfi),
            NiftiType::Uint64 => Self::to_f32::<u64>(nitfi),
            _ => panic!("Unsupported Nifti format: {:?}", ty),
        };

        Self {
            data,
            name,
            transform,
            dim,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn transform(&self) -> Mat4 {
        self.transform
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }

    pub fn size(&self) -> UVec3 {
        UVec3::new(self.dim[0] as u32, self.dim[1] as u32, self.dim[2] as u32)
    }

    fn to_f32<T: DataElement + ToPrimitive>(
        nifti: GenericNiftiObject<InMemNiftiVolume>,
    ) -> Vec<f32> {
        let data: Vec<f32> = nifti
            .into_volume()
            .into_nifti_typed_data::<T>()
            .unwrap()
            .into_iter()
            .filter_map(|voxel| voxel.to_f32())
            .collect();

        if let Some((&min, &max)) = data.iter().minmax().into_option() {
            if min < 0.0 {
                let scale = min.abs().max(max.abs());

                data.into_iter().map(|voxel| voxel / scale).collect()
            } else {
                data.into_iter()
                    .map(|voxel| (voxel - min) / (max - min))
                    .collect()
            }
        } else {
            data
        }
    }
}
