use glam::{Vec3, Vec4};

use std::{collections::HashMap, fs, io::BufRead, path::Path, vec};

use super::bounds::Bounds;

#[derive(Debug, Default)]
pub struct LineFile {
    vertices: Vec<Vec4>,
    indices: Vec<u32>,
    line_counts: Vec<u32>,
    line_offsets: Vec<u32>,
    bounds: Bounds,
}

impl LineFile {
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

    pub fn from_obj(data: &String) -> LineFile {
        let mut lines: Vec<Vec<Vec3>> = vec![vec![]];

        for line in data.lines() {
            match line.split_once(" ") {
                Some(("v", vertex)) => {
                    let mut v = vertex.split_whitespace();

                    lines.last_mut().unwrap().push(Vec3::new(
                        v.next().unwrap().parse().unwrap(),
                        v.next().unwrap().parse().unwrap(),
                        v.next().unwrap().parse().unwrap(),
                    ));
                }
                Some(("l", _)) => {
                    lines.push(Vec::new());
                }
                _ => {}
            }
        }

        Self::from_lines(lines)
    }

    pub fn from_tck(bytes: &[u8], stride: usize) -> LineFile {
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
            lines
                .split(|vertex| !vertex.is_finite())
                .map(|x| x.iter().step_by(stride).copied().collect())
                .collect(),
        )
    }

    pub fn from_lines(lines: Vec<Vec<Vec3>>) -> Self {
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
            bounds: Bounds::from_vertices(&vertices),
            vertices,
            indices,
            line_counts,
            line_offsets,
        }
    }

    pub fn join(tcks: Vec<LineFile>) -> LineFile {
        tcks.into_iter().fold(LineFile::default(), |x, y| {
            let n_vertices = x.vertices.len() as u32;
            let n_indices = x.indices.len() as u32;

            LineFile {
                indices: [
                    x.indices,
                    y.indices.iter().map(|i| i + n_vertices).collect(),
                ]
                .concat(),
                line_offsets: [
                    x.line_offsets,
                    y.line_offsets.iter().map(|i| i + n_indices).collect(),
                ]
                .concat(),
                vertices: [x.vertices, y.vertices].concat(),
                line_counts: [x.line_counts, y.line_counts].concat(),
                bounds: Bounds {
                    min: y.bounds.min.min(x.bounds.min),
                    max: y.bounds.max.max(x.bounds.max),
                },
            }
        })
    }

    pub fn from_tck_file(path: &str, stride: usize) -> LineFile {
        LineFile::from_tck(
            &fs::read(path).expect(&format!("Couldn't read file {:?}", path)),
            stride,
        )
    }

    pub fn from_file(path: &str) -> LineFile {
        if let Some(extension) = Path::new(path).extension() {
            return match extension.to_str() {
                Some("tck") => LineFile::from_tck(
                    &fs::read(path).expect(&format!("Couldn't read file {:?}", path)),
                    1,
                ),
                Some("obj") => LineFile::from_obj(
                    &fs::read_to_string(path).expect(&format!("Couldn't read file {:?}", path)),
                ),
                _ => panic!("unknown file type"),
            };
        }

        panic!("")
    }
}
