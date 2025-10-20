use std::io::Cursor;

use flate2::read::GzDecoder;
use glam::Mat4;
use nifti::{InMemNiftiObject, NiftiObject};

use crate::file::File;

pub struct VolumeFile {
    name: String,
    transform: Mat4,
    data: Vec<f32>,
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

        let data: Vec<f32> = obj.into_volume().into_nifti_typed_data().unwrap();

        Self {
            name: file.name.to_owned(),
            transform,
            data,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn transform(&self) -> Mat4 {
        self.transform
    }

    fn data(&self) -> &[f32] {
        &self.data
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

        dbg!(volume.data);
    }
}
