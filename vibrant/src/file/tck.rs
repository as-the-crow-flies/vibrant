use glam::Vec3;
use std::{collections::HashMap, fs, io::BufRead};

#[derive(Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn scale(&self) -> f32 {
        2.0 * self.min.abs().max_element().max(self.max.max_element())
    }

    pub fn from_vertices(vertices: &[Vec3]) -> Bounds {
        vertices.iter().filter(|&vertex| vertex.is_finite()).fold(
            Bounds {
                min: Vec3::MAX,
                max: Vec3::MIN,
            },
            |bounds, vertex| Bounds {
                min: bounds.min.min(*vertex),
                max: bounds.max.max(*vertex),
            },
        )
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            min: Vec3::MAX,
            max: Vec3::MIN,
        }
    }
}

#[derive(Debug, Default)]
pub struct Tck {
    vertices: Vec<Vec3>,
    caps: Vec<Vec3>,
    bounds: Bounds,
}

impl Tck {
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    pub fn caps(&self) -> &[Vec3] {
        &self.caps
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn from_bytes(bytes: &[u8]) -> Tck {
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
            .expect("Couldnt parse 'file' entry in .tck header as usize");

        let vertices: Vec<Vec3> = bytemuck::try_cast_slice(&bytes[offset..])
            .map(|slice| slice.to_vec())
            // Fallback to copy when vertices are not aligned properly
            .unwrap_or_else(|_| bytemuck::cast_slice(&bytes[offset..].to_owned()).to_vec());

        let bounds = Bounds::from_vertices(&vertices);

        let cap_indices: Vec<usize> = [0, 1]
            .into_iter()
            .chain(
                vertices
                    .iter()
                    .enumerate()
                    .filter_map(|(index, vertex)| vertex.is_nan().then_some(index))
                    .map(|index| [index - 1, index - 2, index + 1, index + 2])
                    .flatten(),
            )
            .collect();

        let caps = cap_indices
            .iter()
            .take(cap_indices.len() - 2)
            .map(|&index| vertices[index])
            .collect();

        Tck {
            vertices,
            caps,
            bounds,
        }
    }

    pub fn join(tcks: Vec<Tck>) -> Tck {
        tcks.into_iter().fold(Tck::default(), |x, y| Tck {
            caps: [x.caps, y.caps].concat(),
            vertices: [x.vertices, y.vertices].concat(),
            bounds: Bounds {
                min: y.bounds.min.min(x.bounds.min),
                max: y.bounds.max.max(x.bounds.max),
            },
        })
    }

    pub fn from_file(path: &str) -> Tck {
        Self::from_bytes(&fs::read(path).expect(&format!("Couldn't read file {:?}", path)))
    }
}
