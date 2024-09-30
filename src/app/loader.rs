use std::{collections::HashMap, io::BufRead, u32};

use bytemuck::checked::cast_slice;
use glam::Vec3;
use itertools::Itertools;
use rfd::AsyncFileDialog;

#[derive(Debug, Default)]
pub struct Tractogram {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}

impl Tractogram {
    pub async fn from_file_dialog() -> Option<Tractogram> {
        let file = AsyncFileDialog::new()
            .add_filter("Tracks file format", &[".tck"])
            .pick_file()
            .await?;

        let bytes = file.read().await;

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
            .expect(".tck header 'file' entry was expected to have '. ' prefix")
            .parse()
            .expect("");

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

        Some(Tractogram { vertices, indices })
    }
}
