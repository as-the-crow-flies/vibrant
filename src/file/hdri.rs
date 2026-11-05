use std::io::Cursor;

use exr::prelude::*;
use glam::UVec2;

use crate::file::File;

#[derive(Debug)]
struct HdriFileData {
    resolution: UVec2,
    pixels: Vec<[f32; 3]>,
}

#[derive(Debug)]
pub struct HdriFile {
    name: String,
    data: HdriFileData,
}

impl HdriFile {
    pub fn from_exr(file: &File) -> Self {
        Self {
            name: file.name.to_owned(),
            data: read()
                .no_deep_data()
                .largest_resolution_level()
                .rgb_channels(
                    |resolution, _| HdriFileData {
                        resolution: UVec2::new(resolution.x() as u32, resolution.y() as u32),
                        pixels: vec![[0.0; 3]; resolution.x() * resolution.y()],
                    },
                    // f32, so sun pixels beyond the f16 range keep their energy.
                    |data, index, (r, g, b): (f32, f32, f32)| {
                        data.pixels[data.resolution.x as usize * index.y() + index.x()] = [r, g, b]
                    },
                )
                .first_valid_layer()
                .all_attributes()
                .from_buffered(Cursor::new(&file.data))
                .unwrap()
                .layer_data
                .channel_data
                .pixels,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data(&self) -> &[[f32; 3]] {
        &self.data.pixels
    }

    pub fn size(&self) -> UVec2 {
        self.data.resolution
    }
}
