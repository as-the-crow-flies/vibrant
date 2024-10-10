use glam::Vec3;
use itertools::Itertools;
use std::{collections::HashMap, fs, io::BufRead};

#[derive(Debug, Default)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn scale(&self) -> f32 {
        2.0 * self.min.abs().max_element().max(self.max.max_element())
    }
}

impl Bounds {
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

#[derive(Debug, Default)]
pub struct Tractogram {
    vertices: Vec<Vec3>,
    bounds: Bounds,
}

impl Tractogram {
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Tractogram {
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

        Tractogram { vertices, bounds }
    }

    pub fn join(tractograms: Vec<Tractogram>) -> Tractogram {
        Tractogram {
            bounds: tractograms.iter().fold(
                Bounds {
                    min: Vec3::MAX,
                    max: Vec3::MIN,
                },
                |bounds, tractogram| Bounds {
                    min: bounds.min.min(tractogram.bounds.min),
                    max: bounds.max.max(tractogram.bounds.max),
                },
            ),
            vertices: tractograms
                .into_iter()
                .map(|tractogram| tractogram.vertices)
                .concat(),
        }
    }

    pub fn from_file(path: &str) -> Tractogram {
        Self::from_bytes(fs::read(path).expect(&format!("Couldn't read file {:?}", path)))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn file_dialog(callback: impl FnOnce(Tractogram) + 'static) {
        wasm_bindgen_futures::spawn_local(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("Tracks file format", &[".tck"])
                .pick_file()
                .await;

            if let Some(file) = file {
                callback(Self::from_bytes(file.read().await));
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn file_dialog(callback: impl FnOnce(Tractogram) + 'static) {
        use itertools::Itertools;

        let tractogram: Option<Tractogram> = rfd::FileDialog::new()
            .add_filter("Tracks file format", &[".tck"])
            .pick_files()
            .map(|files| {
                Tractogram::join(
                    files
                        .iter()
                        .map(|file| Self::from_bytes(fs::read(file).unwrap()))
                        .collect_vec(),
                )
            });

        if let Some(tractogram) = tractogram {
            callback(tractogram);
        }
    }
}
