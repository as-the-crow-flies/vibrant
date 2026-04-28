use std::io::Cursor;

use exr::prelude::*;
use glam::UVec2;

use crate::file::File;

#[derive(Debug)]
struct HdriFileData {
    resolution: UVec2,
    pixels: Vec<[half::f16; 4]>,
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
                        pixels: vec![
                            [f16::ZERO, f16::ZERO, f16::ZERO, f16::ZERO];
                            resolution.x() * resolution.y()
                        ],
                    },
                    |data, index, (r, g, b)| {
                        data.pixels[data.resolution.x as usize * index.y() + index.x()] =
                            [r, g, b, f16::ONE]
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

    pub fn data(&self) -> &[[f16; 4]] {
        &self.data.pixels
    }

    pub fn size(&self) -> UVec2 {
        self.data.resolution
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use crate::file::{hdri::HdriFile, File};

    #[test]
    fn test() {
        let file = HdriFile::from_exr(&File::new(
            "test.exr",
            fs::read("/Users/bkraaijeveld/Data/photo_studio_loft_hall_4k.exr").unwrap(),
        ));

        dbg!(file);
    }
}
