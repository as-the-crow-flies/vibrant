use glam::{Vec3, Vec4};

use bytemuck::{checked::cast_slice, try_cast_slice};
use std::{collections::HashMap, io::BufRead};

use crate::file::File;

use super::bounds::Bounds;

#[derive(Debug, Default)]
pub struct LineFile {
    name: String,
    lines: Vec<Vec<Vec4>>,
    bounds: Bounds,
}

impl LineFile {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn lines(&self) -> &Vec<Vec<Vec4>> {
        &self.lines
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn from_obj(file: File) -> LineFile {
        let mut lines: Vec<Vec<Vec4>> = vec![vec![]];

        let name = file.name.replace(".obj", "");

        let data = String::from_utf8(file.data).unwrap();

        for line in data.lines() {
            match line.split_once(" ") {
                Some(("v", vertex)) => {
                    let mut v = vertex.split_whitespace();

                    lines.last_mut().unwrap().push(Vec4::new(
                        v.next().unwrap().parse().unwrap(),
                        v.next().unwrap().parse().unwrap(),
                        v.next().unwrap().parse().unwrap(),
                        1.0,
                    ));
                }
                Some(("l", _)) => {
                    lines.push(Vec::new());
                }
                _ => {}
            }
        }

        lines.pop();

        Self {
            name,
            bounds: Bounds::from_vertices(lines.iter().flatten()),
            lines,
        }
    }

    pub fn from_tck(file: File) -> LineFile {
        let bytes = file.data();

        let header = TckHeader::parse(bytes);

        let lines: Vec<Vec3> = try_cast_slice(&bytes[header.offset..])
            .map(|slice| slice.to_vec())
            // Fallback to copy when vertices are not aligned properly
            .unwrap_or_else(|_| cast_slice(&bytes[header.offset..].to_owned()).to_vec());

        let lines: Vec<Vec<Vec4>> = lines
            .split(|vertex| !vertex.is_finite())
            .filter(|line| line.len() >= 2)
            .map(|line| line.iter().map(|v| Vec4::new(v.x, v.y, v.z, 1.0)).collect())
            .collect();

        Self {
            name: file.name.replace(".tck", ""),
            bounds: Bounds::from_vertices(lines.iter().flatten()),
            lines,
        }
    }
}

pub struct TrackScalarFile {
    name: String,
    values: Vec<f32>,
}

impl TrackScalarFile {
    pub fn from_tsf(file: File) -> TrackScalarFile {
        let bytes = file.data();

        let header = TckHeader::parse(bytes);

        let values = cast_slice(&bytes[header.offset..])
            .split(|value: &f32| !value.is_finite())
            .filter(|line| line.len() >= 2)
            .flatten()
            .copied()
            .collect();

        Self {
            name: file.name,
            values,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn values(&self) -> &Vec<f32> {
        &self.values
    }
}

struct TckHeader {
    offset: usize,
}

impl TckHeader {
    pub fn parse(data: &[u8]) -> Self {
        let header: HashMap<String, String> = data
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

        Self { offset }
    }
}
