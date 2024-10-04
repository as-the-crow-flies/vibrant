use glam::Vec3;
use std::{collections::HashMap, fs, io::BufRead, u32};

#[derive(Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn scale(&self) -> f32 {
        self.min.abs().max_element().max(self.max.max_element())
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

#[derive(Debug)]
pub struct Tractogram {
    vertices: Vec<Vec3>,
    indices: Vec<u32>,
    bounds: Bounds,
}

impl Tractogram {
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
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

        let vertices: Vec<Vec3> = bytemuck::cast_slice(&bytes[offset..]).to_vec();

        let indices: Vec<u32> = vertices
            .iter()
            .scan(0u32, |index, vertex| {
                let result = if vertex.is_finite() { *index } else { u32::MAX };

                if vertex.is_finite() {
                    *index += 1;
                }

                return Some(result);
            })
            .collect();

        let bounds = Bounds::from_vertices(&vertices);

        Tractogram {
            vertices,
            indices,
            bounds,
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
        let file = rfd::FileDialog::new()
            .add_filter("Tracks file format", &[".tck"])
            .pick_file();

        if let Some(file) = file {
            callback(Self::from_bytes(fs::read(file).unwrap()))
        }
    }
}
