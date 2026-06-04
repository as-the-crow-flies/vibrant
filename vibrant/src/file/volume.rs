use std::io::Cursor;

use flate2::read::GzDecoder;
use glam::{Mat4, UVec3, Vec3, Vec4};
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
    size: UVec3,
}

impl VolumeFile {
    pub fn from_comressed_nifti(file: &File) -> Self {
        Self::from_nifti_obj(
            file.name.replace(".nii.gz", "").to_owned(),
            InMemNiftiObject::from_reader(GzDecoder::new(Cursor::new(&file.data)))
                .expect("Nifti should contain volume data"),
        )
    }

    pub fn from_nifti(file: &File) -> Self {
        Self::from_nifti_obj(
            file.name.replace(".nii", "").to_owned(),
            InMemNiftiObject::from_reader(Cursor::new(&file.data))
                .expect("Nifti should contain volume data"),
        )
    }

    fn from_nifti_obj(name: String, nifti: InMemNiftiObject) -> Self {
        let dim = nifti
            .header()
            .dim()
            .expect("Nifti should contain valid dimensionality");

        let voxel_to_texture = Mat4::from_scale(Vec3::new(
            1.0 / dim[0] as f32,
            1.0 / dim[1] as f32,
            1.0 / dim[2] as f32,
        ));

        let left_right_flip = Mat4::from_diagonal(Vec4::new(-1.0, 1.0, 1.0, 1.0));

        let mm_to_voxel = Mat4::from_cols_array_2d(&nifti.header().affine().into()).inverse();

        let transform = voxel_to_texture * mm_to_voxel * left_right_flip;

        let dim = nifti
            .header()
            .dim()
            .expect("Invalid Nifti dimension")
            .to_vec();

        let size = UVec3::new(dim[0] as u32, dim[1] as u32, dim[2] as u32);

        let ty = nifti.header().data_type().expect("Invalid Nifti data type");

        let data: Vec<f32> = match ty {
            NiftiType::Uint8 => Self::to_f32::<u8>(nifti),
            NiftiType::Int16 => Self::to_f32::<i16>(nifti),
            NiftiType::Int32 => Self::to_f32::<i32>(nifti),
            NiftiType::Float32 => Self::to_f32::<f32>(nifti),
            NiftiType::Float64 => Self::to_f32::<f64>(nifti),
            NiftiType::Int8 => Self::to_f32::<i8>(nifti),
            NiftiType::Uint16 => Self::to_f32::<u16>(nifti),
            NiftiType::Uint32 => Self::to_f32::<u32>(nifti),
            NiftiType::Int64 => Self::to_f32::<i64>(nifti),
            NiftiType::Uint64 => Self::to_f32::<u64>(nifti),
            _ => panic!("Unsupported Nifti format: {:?}", ty),
        };

        Self {
            data,
            name,
            transform,
            size,
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
        self.size
    }

    pub fn is_binary(&self) -> bool {
        self.data.iter().all(|&x| x == 0.0 || x == 1.0)
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
