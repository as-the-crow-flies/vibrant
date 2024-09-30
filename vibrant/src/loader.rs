use glam::Vec3;
use rfd::AsyncFileDialog;
use std::{collections::HashMap, fs, io::BufRead, u32};

#[derive(Debug, Default)]
pub struct Tractogram {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}

impl Tractogram {
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

        Tractogram { vertices, indices }
    }

    pub fn from_file(path: &str) -> Tractogram {
        Self::from_bytes(fs::read(path).expect(&format!("Couldn't read file {:?}", path)))
    }

    pub async fn from_file_dialog() -> Option<Tractogram> {
        let file = AsyncFileDialog::new()
            .add_filter("Tracks file format", &[".tck"])
            .pick_file()
            .await?;

        Some(Self::from_bytes(file.read().await))
    }
}
