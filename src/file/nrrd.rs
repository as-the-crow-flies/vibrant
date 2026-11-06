use std::{borrow::Cow, collections::HashMap, io::Read};

use flate2::read::GzDecoder;
use glam::{Mat4, UVec3, Vec3, Vec4};

use crate::file::{volume::Scalar, File, VolumeFile};

impl VolumeFile {
    /// Single-file NRRD with attached data.
    pub fn from_nrrd(file: &File) -> Self {
        let name = file.name().trim_end_matches(".nrrd").to_owned();
        let (header, offset) = Header::parse(file.data());

        header.into_volume(name, &file.data()[offset..])
    }

    /// Detached NRRD: `.nhdr` header plus the data file it references.
    pub fn from_nhdr(header: &File, data: &File) -> Self {
        let name = header.name().trim_end_matches(".nhdr").to_owned();
        let (header, _) = Header::parse(header.data());

        header.into_volume(name, data.data())
    }
}

/// File name of the data file a `.nhdr` references.
pub fn nhdr_data_file(header: &File) -> String {
    let (header, _) = Header::parse(header.data());
    let path = header
        .get("data file")
        .or_else(|| header.get("datafile"))
        .expect("NHDR should reference a data file");

    assert!(
        !path.starts_with("LIST") && path.split_whitespace().count() == 1,
        "Multi-file NRRD data is not supported"
    );

    path.rsplit(['/', '\\']).next().unwrap().to_owned()
}

struct Header {
    fields: HashMap<String, String>,
}

impl Header {
    /// Returns the header and the byte offset where attached data starts.
    fn parse(bytes: &[u8]) -> (Self, usize) {
        assert!(bytes.starts_with(b"NRRD"), "Not a NRRD file");

        let mut fields = HashMap::new();
        let mut offset = 0;

        for line in bytes.split_inclusive(|&b| b == b'\n') {
            offset += line.len();
            let line = String::from_utf8_lossy(line);
            let line = line.trim_end_matches(['\n', '\r']);

            if line.is_empty() {
                break;
            }
            if line.starts_with('#') || line.starts_with("NRRD") {
                continue;
            }
            if let Some((key, value)) = line.split_once(": ") {
                fields.insert(key.trim().to_lowercase(), value.trim().to_owned());
            }
        }

        (Self { fields }, offset)
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    fn into_volume(self, name: String, bytes: &[u8]) -> VolumeFile {
        let sizes: Vec<u32> = self
            .get("sizes")
            .expect("NRRD should have sizes")
            .split_whitespace()
            .map(|size| size.parse().expect("Invalid NRRD size"))
            .collect();
        let [x, y, z] = sizes[..] else {
            panic!("Only 3D NRRD volumes are supported, got sizes {sizes:?}");
        };
        let size = UVec3::new(x, y, z);

        let encoding = self.get("encoding").unwrap_or("raw");
        let bytes = match encoding {
            "raw" => Cow::Borrowed(bytes),
            "gzip" | "gz" => {
                let mut out = Vec::new();
                GzDecoder::new(bytes)
                    .read_to_end(&mut out)
                    .expect("Invalid gzip NRRD data");
                Cow::Owned(out)
            }
            _ => panic!("Unsupported NRRD encoding: {encoding}"),
        };

        let scalar = scalar(self.get("type").expect("NRRD should have a type"));
        let little = self.get("endian") != Some("big");
        let voxels = scalar.decode(&bytes, little, size);

        VolumeFile::from_raw(name, size, self.affine(), voxels)
    }

    /// Voxel index to RAS mm.
    fn affine(&self) -> Mat4 {
        let to_ras = match self.get("space").map(str::to_lowercase).as_deref() {
            Some("left-posterior-superior" | "lps") => Vec3::new(-1.0, -1.0, 1.0),
            Some("left-anterior-superior" | "las") => Vec3::new(-1.0, 1.0, 1.0),
            _ => Vec3::ONE,
        };

        let directions = self
            .get("space directions")
            .map(|value| vectors(value).collect::<Vec<_>>())
            .filter(|directions| directions.len() == 3)
            .unwrap_or_else(|| {
                let spacings: Vec<f32> = self
                    .get("spacings")
                    .map(|value| {
                        value
                            .split_whitespace()
                            .map(|s| {
                                s.parse()
                                    .ok()
                                    .filter(|s: &f32| s.is_finite())
                                    .unwrap_or(1.0)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let spacing = |i: usize| spacings.get(i).copied().unwrap_or(1.0);
                vec![
                    Vec3::X * spacing(0),
                    Vec3::Y * spacing(1),
                    Vec3::Z * spacing(2),
                ]
            });

        let origin = self
            .get("space origin")
            .and_then(|value| vectors(value).next())
            .unwrap_or(Vec3::ZERO);

        Mat4::from_cols(
            (directions[0] * to_ras).extend(0.0),
            (directions[1] * to_ras).extend(0.0),
            (directions[2] * to_ras).extend(0.0),
            Vec4::from((origin * to_ras, 1.0)),
        )
    }
}

/// Parses `(x,y,z) (x,y,z) ...`.
fn vectors(value: &str) -> impl Iterator<Item = Vec3> + '_ {
    value.split(')').filter_map(|vector| {
        let components: Vec<f32> = vector
            .trim()
            .trim_start_matches('(')
            .split(',')
            .filter_map(|c| c.trim().parse().ok())
            .collect();
        (components.len() == 3).then(|| Vec3::from_slice(&components))
    })
}

fn scalar(ty: &str) -> Scalar {
    match ty.trim_end_matches("_t") {
        "signed char" | "int8" => Scalar::I8,
        "uchar" | "unsigned char" | "uint8" => Scalar::U8,
        "short" | "short int" | "signed short" | "signed short int" | "int16" => Scalar::I16,
        "ushort" | "unsigned short" | "unsigned short int" | "uint16" => Scalar::U16,
        "int" | "signed int" | "int32" => Scalar::I32,
        "uint" | "unsigned int" | "uint32" => Scalar::U32,
        "longlong"
        | "long long"
        | "long long int"
        | "signed long long"
        | "signed long long int"
        | "int64" => Scalar::I64,
        "ulonglong" | "unsigned long long" | "unsigned long long int" | "uint64" => Scalar::U64,
        "float" => Scalar::F32,
        "double" => Scalar::F64,
        _ => panic!("Unsupported NRRD type: {ty}"),
    }
}
