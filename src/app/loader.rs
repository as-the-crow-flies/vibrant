use std::{iter::once, u32};

use bytemuck::cast_slice;
use glam::Vec3;

#[derive(Debug)]
pub struct Tractogram {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}

pub fn load_tck(bytes: &[u8]) -> Tractogram {
    let offset = bytes
        .windows(4)
        .position(|w| w == b"END\n")
        .map(|ix| ix + 4)
        .expect(".tck file does not contain 'END'");

    let payload = bytes[offset..].to_owned();

    let vertices: Vec<Vec<Vec3>> = cast_slice(&payload)
        .split(|x: &Vec3| x.is_nan() || !x.is_finite())
        .map(|x| x.into())
        .collect();

    let indices: Vec<u32> = vertices
        .iter()
        .map(|streamline| streamline.len() as u32)
        .scan(0, |end, length| {
            let start = *end;
            *end += length;
            Some(start..*end)
        })
        .map(|indices| indices.chain(once(u32::MAX)))
        .flatten()
        .collect();

    let vertices: Vec<Vec3> = vertices
        .into_iter()
        .flatten()
        .map(|vertex| Vec3::new(vertex.y, vertex.z, vertex.x))
        .collect();

    Tractogram { vertices, indices }
}
