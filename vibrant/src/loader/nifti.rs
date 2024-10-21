use std::{f32::consts::PI, io::Read};

use bytemuck::{cast_slice, Pod, Zeroable};
use flate2::bufread::GzDecoder;
use glam::{Mat4, Quat, Vec3, Vec4};
use itertools::{Itertools, MinMaxResult};

pub struct Nifti {
    width: u32,
    height: u32,
    depth: u32,
    transform: Mat4,
    data: Vec<f32>,
    histogram: [f32; 256],
}

impl Nifti {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn transform(&self) -> Mat4 {
        self.transform
    }

    pub fn data(&self) -> &[u8] {
        bytemuck::cast_slice(&self.data)
    }

    pub fn histogram(&self) -> &[u8] {
        bytemuck::cast_slice(&self.histogram)
    }
}

#[repr(C)]
#[derive(Debug, Pod, Zeroable, Clone, Copy)]
struct NiftiRawHeader {
    sizeof_hdr: i32,       // Size of the header. Must be 348 (bytes).
    data_type: [u8; 10],   // Not used; compatibility with analyze.
    db_name: [u8; 18],     // Not used; compatibility with analyze.
    extents: i32,          // Not used; compatibility with analyze.
    session_error: i16,    // Not used; compatibility with analyze.
    regular: u8,           // Not used; compatibility with analyze.
    dim_info: u8,          // Encoding directions (phase, frequency, slice).
    dim: [i16; 8],         // Data array dimensions.
    intent_p1: f32,        // 1st intent parameter.
    intent_p2: f32,        // 2nd intent parameter.
    intent_p3: f32,        // 3rd intent parameter.
    intent_code: i16,      // nifti intent.
    datatype: i16,         // Data type.
    bitpix: i16,           // Number of bits per voxel.
    slice_start: i16,      // First slice index.
    pixdim: [f32; 8],      // Grid spacings (unit per dimension).
    vox_offset: f32,       // Offset into a .nii file.
    scl_slope: f32,        // Data scaling, slope.
    scl_inter: f32,        // Data scaling, offset.
    slice_end: i16,        // Last slice index.
    slice_code: u8,        // Slice timing order.
    xyzt_units: u8,        // Units of pixdim[1..4].
    cal_max: f32,          // Maximum display intensity.
    cal_min: f32,          // Minimum display intensity.
    slice_duration: f32,   // Time for one slice.
    toffset: f32,          // Time axis shift.
    glmax: i32,            // Not used; compatibility with analyze.
    glmin: i32,            // Not used; compatibility with analyze.
    descrip: [u8; 80],     // Any text.
    aux_file: [u8; 24],    // Auxiliary filename.
    qform_code: i16,       // Use the quaternion fields.
    sform_code: i16,       // Use of the affine fields.
    quatern_b: f32,        // Quaternion b parameter.
    quatern_c: f32,        // Quaternion c parameter.
    quatern_d: f32,        // Quaternion d parameter.
    qoffset_x: f32,        // Quaternion x shift.
    qoffset_y: f32,        // Quaternion y shift.
    qoffset_z: f32,        // Quaternion z shift.
    srow_x: [f32; 4],      // 1st row affine transform
    srow_y: [f32; 4],      // 2nd row affine transform.
    srow_z: [f32; 4],      // 3rd row affine transform.
    intent_name: [u8; 16], // Name or meaning of the data.
    magic: [u8; 4],        // Magic string.
}

impl NiftiRawHeader {
    const SIZE: usize = 348;

    pub fn data_offset(&self) -> usize {
        (self.vox_offset as usize) - NiftiRawHeader::SIZE
    }

    pub fn size(&self) -> Vec3 {
        Vec3::new(self.dim[1] as f32, self.dim[2] as f32, self.dim[3] as f32)
    }

    pub fn rotation(&self) -> Quat {
        let quatern_a =
            (1.0 - self.quatern_b.powi(2) - self.quatern_c.powi(2) - self.quatern_d.powi(2)).sqrt();

        Quat::from_xyzw(quatern_a, self.quatern_b, self.quatern_c, self.quatern_d).inverse()
            * Quat::from_rotation_x(PI / 2.0)
    }

    pub fn transform(&self) -> Mat4 {
        let (scale, rotation, translation) = Mat4::from_cols(
            self.srow_x.into(),
            self.srow_y.into(),
            self.srow_z.into(),
            Vec4::W,
        )
        .transpose()
        .inverse()
        .to_scale_rotation_translation();

        Mat4::from_scale_rotation_translation(
            scale * self.size().max_element() / self.size(),
            rotation * Quat::from_rotation_x(0.5 * PI),
            translation / self.size(),
        )
    }

    pub fn data(&self, bytes: &[u8]) -> Vec<f32> {
        let data = &bytes[self.data_offset()..];

        match self.datatype {
            2 => data.into_iter().map(|&x| x as f32).collect(),
            4 => cast_slice::<u8, u16>(data)
                .into_iter()
                .map(|&x| x as f32)
                .collect(),
            16 => cast_slice::<u8, f32>(data).to_vec(),
            64 => cast_slice::<u8, f64>(data)
                .into_iter()
                .map(|&x| x as f32)
                .collect(),
            _ => panic!("Nifti Datatype ({}) not supported", self.datatype),
        }
    }
}

impl Nifti {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut vector = Vec::new();

        GzDecoder::new(bytes)
            .read_to_end(&mut vector)
            .expect("Couldn't decode .nii.gz file");

        let (header, data) = vector.split_at(NiftiRawHeader::SIZE);
        let header: NiftiRawHeader = *bytemuck::from_bytes(header);
        let data = header.data(data);

        dbg!(header);

        Self {
            width: header.dim[1] as u32,
            height: header.dim[2] as u32,
            depth: header.dim[3] as u32,
            transform: header.transform(),
            histogram: Self::compute_histogram(&data),
            data,
        }
    }

    fn compute_histogram(data: &[f32]) -> [f32; 256] {
        let mut result = [0.0; 256];

        if let MinMaxResult::MinMax(min, max) = data.iter().minmax() {
            let range = 1.0 / (max - min) * 255.0;

            for item in data {
                result[((item - min) * range) as usize] += 1.0;
            }
        }

        let result = result.map(|x: f32| x.log10());
        let max = result.into_iter().fold(0.0f32, |x, y| x.max(y));

        return result.map(|x| x / max);
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use glam::Vec4;

    use super::Nifti;

    #[test]
    fn can_load_nifti_file() {
        let nifti = Nifti::from_bytes(
            &fs::read("../assets/HPC-100307/T1w_acpc_dc_restore_1.25.nii.gz").unwrap(),
        );

        dbg!(nifti.transform.to_scale_rotation_translation());

        dbg!(nifti.transform * Vec4::new(-0.5, -0.5, -0.5, 1.0));
        dbg!(nifti.transform * Vec4::new(0.0, 0.0, 0.0, 1.0));
        dbg!(nifti.transform * Vec4::new(0.5, 0.5, 0.5, 1.0));

        dbg!(
            nifti.transform
                * Vec4::new(
                    nifti.width() as f32,
                    nifti.height() as f32,
                    nifti.depth() as f32,
                    1.0
                )
        );
    }
}
