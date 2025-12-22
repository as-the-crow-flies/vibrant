use std::io::Cursor;

use flate2::read::GzDecoder;
use glam::Mat4;
use nifti::{InMemNiftiObject, NiftiObject, NiftiType};

use crate::file::File;

pub enum VolumeType {
    Uint8,
    Uint16,
    Uint32,
    Int8,
    Int16,
    Int32,
    Float32,
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

        let transform = Mat4::from_cols_array_2d(&[
            obj.header().srow_x,
            obj.header().srow_y,
            obj.header().srow_z,
            [0.0, 0.0, 0.0, 1.0],
        ])
        .transpose();

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

    pub fn ty(&self) -> &VolumeType {
        &self.ty
    }

    pub fn dim(&self) -> &[u16] {
        &self.dim
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use crate::file::{volume::VolumeFile, File};

    #[test]
    fn can_read_nifti_file() {
        let file = File {
            name: "T1w_acpc_dc_restore_1.25.nii.gz".to_owned(),
            data: fs::read("/Users/bkraaijeveld/Projects/vibrant/assets/HCP-100307/T1w_acpc_dc_restore_1.25.nii.gz")
                .expect("Should be able to load nifti file"),
        };

        let volume = VolumeFile::from_nifti(&file);

        dbg!(volume.transform());
    }

    #[test]
    fn can_read_integer_nifti_file() {
        let file = File {
            name: "T1w_acpc_dc_restore_1.25.nii.gz".to_owned(),
            data: fs::read("/Users/bkraaijeveld/Projects/vibrant/assets/HCP-100307/m2m_hcp-100307/final_tissues.nii.gz")
                .expect("Should be able to load nifti file"),
        };

        let volume = VolumeFile::from_nifti(&file);

        dbg!(volume.transform());
    }
}
