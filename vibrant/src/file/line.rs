use glam::{Vec3, Vec4};
use itertools::Itertools;

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

        // Self::orient_lines(&mut lines);

        Self {
            name,
            bounds: Bounds::from_vertices(lines.iter().flatten()),
            lines,
        }
    }

    pub fn from_tck(file: File) -> LineFile {
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

        let mut lines: Vec<Vec<Vec4>> = lines
            .split(|vertex| !vertex.is_finite())
            .filter(|line| line.len() >= 2)
            .map(|line| line.iter().map(|v| Vec4::new(v.x, v.y, v.z, 1.0)).collect())
            .collect();

        Self::orient_lines(&mut lines);

        Self {
            name: file.name.replace(".tck", ""),
            bounds: Bounds::from_vertices(lines.iter().flatten()),
            lines,
        }
    }

    fn orient_lines(lines: &mut Vec<Vec<Vec4>>) {
        lines.sort_by_key(|line| -(line.len() as i32));

        let mut reference = lines.first().unwrap().iter().copied().collect_vec();

        Self::orient_line(&mut reference, Vec4::new(0.0, 0.0, 1.0, 0.0));

        for line in lines {
            Self::orient_to_reference(line, &reference);
        }
    }

    fn orient_line(line: &mut Vec<Vec4>, preferred: Vec4) {
        let dir = line.last().unwrap() - line.first().unwrap();
        if dir.dot(preferred) < 0.0 {
            line.reverse();
        }
    }

    fn orient_to_reference(line: &mut Vec<Vec4>, reference: &Vec<Vec4>) {
        let d_forward = line.first().unwrap().distance(*reference.first().unwrap())
            + line.last().unwrap().distance(*reference.last().unwrap());

        let d_reverse = line.first().unwrap().distance(*reference.last().unwrap())
            + line.last().unwrap().distance(*reference.first().unwrap());

        if d_reverse < d_forward {
            line.reverse();
        }
    }
}
