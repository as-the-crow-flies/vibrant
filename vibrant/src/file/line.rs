use glam::{Vec3, Vec4};

use std::{collections::HashMap, io::BufRead};

use crate::file::File;

use super::bounds::Bounds;

#[derive(Debug, Default)]
pub struct LineFile {
    name: String,
    vertices: Vec<Vec4>,
    indices: Vec<u32>,
    line_counts: Vec<u32>,
    line_offsets: Vec<u32>,
    bounds: Bounds,
}

impl LineFile {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn vertices(&self) -> &[Vec4] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn line_counts(&self) -> &[u32] {
        &self.line_counts
    }

    pub fn line_offsets(&self) -> &[u32] {
        &self.line_offsets
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn from_tck(file: &File) -> LineFile {
        let bytes = file.data();

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

        let lines: Vec<Vec3> = bytemuck::try_cast_slice(&bytes[offset..])
            .map(|slice| slice.to_vec())
            // Fallback to copy when vertices are not aligned properly
            .unwrap_or_else(|_| bytemuck::cast_slice(&bytes[offset..].to_owned()).to_vec());

        Self::from_lines(
            file.name.to_owned(),
            lines
                .split(|vertex| !vertex.is_finite())
                .map(|x| x.iter().copied().collect())
                .collect(),
        )
    }

    pub fn from_lines(name: String, lines: Vec<Vec<Vec3>>) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut line_counts = Vec::new();

        for line in lines.iter().filter(|line| line.len() >= 2) {
            let index_length = line.len() - 1;

            indices.extend((vertices.len()..vertices.len() + index_length).map(|i| i as u32));
            vertices.extend(line.iter().map(|v| Vec4::new(v.x, v.y, v.z, 1.0)));
            line_counts.push(index_length as u32);
        }

        let line_offsets = line_counts
            .iter()
            .scan(0, |total, c| {
                let result = *total;
                *total += c;
                Some(result)
            })
            .collect();

        Self {
            name,
            bounds: Bounds::from_vertices(&vertices),
            vertices,
            indices,
            line_counts,
            line_offsets,
        }
    }
}
