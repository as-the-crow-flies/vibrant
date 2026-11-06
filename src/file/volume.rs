use std::io::Cursor;

use flate2::read::GzDecoder;
use glam::{Mat4, UVec3, Vec3};
use half::f16;
use nifti::{Endianness, InMemNiftiObject, NiftiObject, NiftiType};

use crate::file::File;

pub struct VolumeFile {
    name: String,
    transform: Mat4,
    voxels: Voxels,
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
        let header = nifti.header();
        let dim = header.dim().expect("Invalid Nifti dimension");
        let size = UVec3::new(dim[0] as u32, dim[1] as u32, dim[2] as u32);
        let affine = Mat4::from_cols_array_2d(&header.affine().into());
        let little = header.endianness == Endianness::Little;

        let ty = header.data_type().expect("Invalid Nifti data type");
        let scalar = match ty {
            NiftiType::Int8 => Scalar::I8,
            NiftiType::Uint8 => Scalar::U8,
            NiftiType::Int16 => Scalar::I16,
            NiftiType::Uint16 => Scalar::U16,
            NiftiType::Int32 => Scalar::I32,
            NiftiType::Uint32 => Scalar::U32,
            NiftiType::Int64 => Scalar::I64,
            NiftiType::Uint64 => Scalar::U64,
            NiftiType::Float32 => Scalar::F32,
            NiftiType::Float64 => Scalar::F64,
            _ => panic!("Unsupported Nifti format: {:?}", ty),
        };

        let bytes = nifti.into_volume().into_raw_data();

        Self::from_raw(name, size, affine, scalar.decode(&bytes, little, size))
    }

    /// `affine` maps voxel indices to RAS mm, like a NIfTI affine.
    pub(super) fn from_raw(name: String, size: UVec3, affine: Mat4, voxels: Voxels) -> Self {
        Self {
            name,
            transform: Mat4::from_scale(Vec3::ONE / size.as_vec3()) * affine.inverse(),
            voxels,
            size,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn transform(&self) -> Mat4 {
        self.transform
    }

    /// Normalized to [0, 1], or [-1, 1] if signed.
    pub fn data(&self) -> &[f16] {
        &self.voxels.data
    }

    pub fn histogram(&self) -> &Bins {
        &self.voxels.histogram
    }

    pub fn size(&self) -> UVec3 {
        self.size
    }

    pub fn is_signed(&self) -> bool {
        self.voxels.signed
    }
}

pub(super) struct Voxels {
    data: Vec<f16>,
    histogram: Bins,
    signed: bool,
}

/// Histogram over the normalized range; bin `i` is centered at
/// `first + i * step`. `density` is voxels per level that occurs in the bin,
/// so levels missing from rescaled data don't make neighboring bins uneven.
pub struct Bins {
    pub density: Vec<f32>,
    pub first: f32,
    pub step: f32,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Scalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Scalar {
    fn is_integer(self) -> bool {
        !matches!(self, Scalar::F32 | Scalar::F64)
    }

    /// Decodes `size` voxels straight into normalized f16, in two passes over
    /// the raw bytes (min/max, then normalize + histogram).
    pub(super) fn decode(self, bytes: &[u8], little: bool, size: UVec3) -> Voxels {
        let count = size.element_product() as usize;
        let integer = self.is_integer();

        macro_rules! decode {
            ($t:ty) => {
                normalize::<{ std::mem::size_of::<$t>() }>(bytes, count, integer, |chunk| {
                    (if little {
                        <$t>::from_le_bytes(chunk)
                    } else {
                        <$t>::from_be_bytes(chunk)
                    }) as f32
                })
            };
        }

        match self {
            Scalar::I8 => decode!(i8),
            Scalar::U8 => decode!(u8),
            Scalar::I16 => decode!(i16),
            Scalar::U16 => decode!(u16),
            Scalar::I32 => decode!(i32),
            Scalar::U32 => decode!(u32),
            Scalar::I64 => decode!(i64),
            Scalar::U64 => decode!(u64),
            Scalar::F32 => decode!(f32),
            Scalar::F64 => decode!(f64),
        }
    }
}

const BINS: u32 = 256;

/// Integer data gets bins of a whole number of levels each, so no bin
/// catches one level more than its neighbor. Unsigned data is normalized to
/// [min, p99.99] so a few bright outliers don't squash the useful range.
fn normalize<const N: usize>(
    bytes: &[u8],
    count: usize,
    integer: bool,
    read: impl Fn([u8; N]) -> f32,
) -> Voxels {
    assert!(bytes.len() >= count * N, "Volume data is truncated");

    let values = || {
        bytes[..count * N]
            .chunks_exact(N)
            .map(|chunk| read(chunk.try_into().unwrap()))
    };

    let (min, max) = values().fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), v| {
        (min.min(v), max.max(v))
    });

    let signed = min < 0.0;
    let (offset, range, distinct) = if signed {
        (0.0, min.abs().max(max.abs()), BINS)
    } else {
        let (max, distinct) = clip(values, min, max, integer);
        (min, max - min, distinct)
    };
    let scale = if range > 0.0 { 1.0 / range } else { 0.0 };

    // f32 holds integers exactly up to 2^24.
    let levels = range + 1.0;
    let aligned = integer && !signed && levels <= 16_777_216.0;
    // At least as many levels per bin as the data skips between present
    // levels, so rescaled data (e.g. 172 levels over 0..255) has no empty bins.
    let per_bin = if aligned {
        (levels as u32).div_ceil(distinct.clamp(1, BINS))
    } else {
        1
    };
    let bins = if aligned {
        (levels as u32).div_ceil(per_bin)
    } else {
        BINS
    };

    let mut counts = vec![0u32; bins as usize];
    let mut present = vec![false; if aligned { levels as usize } else { 0 }];
    let data = values()
        .map(|value| {
            let bin = if aligned {
                let level = ((value - min) as u32).min(range as u32);
                present[level as usize] = true;
                level / per_bin
            } else {
                ((value - offset) * scale * BINS as f32) as u32
            };
            counts[bin.min(bins - 1) as usize] += 1;
            f16::from_f32(((value - offset) * scale).min(1.0))
        })
        .collect();

    let histogram = if aligned {
        let span = range.max(1.0);
        let density = present
            .chunks(per_bin as usize)
            .zip(&counts)
            .map(|(levels, &count)| {
                let levels = levels.iter().filter(|&&present| present).count();
                count as f32 / levels.max(1) as f32
            })
            .collect();
        Bins {
            density,
            first: (per_bin - 1) as f32 * 0.5 / span,
            step: per_bin as f32 / span,
        }
    } else {
        Bins {
            density: counts.iter().map(|&count| count as f32).collect(),
            first: 0.5 / BINS as f32,
            step: 1.0 / BINS as f32,
        }
    };

    Voxels {
        data,
        histogram,
        signed,
    }
}

const CLIP_PERCENTILE: f64 = 0.9999;

/// The 99.99th percentile value and the number of distinct levels up to it,
/// from a 65536-bin histogram over [min, max] (one level per bin for 16-bit).
fn clip<I: Iterator<Item = f32>>(
    values: impl Fn() -> I,
    min: f32,
    max: f32,
    integer: bool,
) -> (f32, u32) {
    const FINE: usize = 65536;

    let range = max - min;
    if range <= 0.0 {
        return (max, 1);
    }

    let scale = FINE as f32 / range;
    let mut fine = vec![0u32; FINE];
    let mut total = 0u64;
    for value in values() {
        fine[(((value - min) * scale) as usize).min(FINE - 1)] += 1;
        total += 1;
    }

    let target = (total as f64 * CLIP_PERCENTILE).ceil() as u64;
    let mut cumulative = 0u64;
    let bin = fine
        .iter()
        .position(|&count| {
            cumulative += count as u64;
            cumulative >= target
        })
        .unwrap_or(FINE - 1);

    // Integers: the level inside fine bin `bin`; floats: its upper edge.
    let clipped = if integer {
        min + (bin as f32 / scale).ceil()
    } else {
        min + (bin + 1) as f32 / scale
    };

    let distinct = |end: usize| fine[..end].iter().filter(|&&count| count > 0).count() as u32;

    // A sparse mask can put p99.99 on the background; keep the full range then.
    if clipped <= min {
        (max, distinct(FINE))
    } else {
        (clipped.min(max), distinct(bin + 1))
    }
}
