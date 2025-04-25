use glam::Vec3;

use std::{collections::HashMap, fs, io::BufRead};

use super::bounds::Bounds;

#[derive(Debug, Default)]
pub struct TractogramFile {
    vertices: Vec<Vec3>,
    bounds: Bounds,
}

impl TractogramFile {
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn from_tck(bytes: &[u8]) -> TractogramFile {
        let header: HashMap<String, String> = bytes
            .lines()
            .map(|line| line.unwrap())
            .take_while(|line| line != "END")
            .filter_map(|line| {
                line.split_once(": ")
                    .map(|(key, value)| (key.to_string(), value.to_string()))
            })
            .collect();

        let offset: usize = header
            .get("file")
            .expect("No 'file' entry in .tck header")
            .strip_prefix(". ")
            .expect("'file' entry in .tck header was expected to have '. ' prefix")
            .parse()
            .expect("Couldn't parse 'file' entry in .tck header as usize");

        let vertices: Vec<Vec3> = bytemuck::try_cast_slice(&bytes[offset..])
            .map(|slice| slice.to_vec())
            // Fallback to copy when vertices are not aligned properly
            .unwrap_or_else(|_| bytemuck::cast_slice(&bytes[offset..].to_owned()).to_vec());

        let bounds = Bounds::from_vertices(&vertices);

        TractogramFile { vertices, bounds }
    }

    pub fn join(tcks: Vec<TractogramFile>) -> TractogramFile {
        tcks.into_iter()
            .fold(TractogramFile::default(), |x, y| TractogramFile {
                vertices: [x.vertices, y.vertices].concat(),
                bounds: Bounds {
                    min: y.bounds.min.min(x.bounds.min),
                    max: y.bounds.max.max(x.bounds.max),
                },
            })
    }

    pub fn from_file(path: &str) -> TractogramFile {
        Self::from_tck(&fs::read(path).expect(&format!("Couldn't read file {:?}", path)))
    }
}
