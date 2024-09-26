use std::{iter::once, u32};

use bytemuck::{cast_slice, Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct Vertex(f32, f32, f32);

impl Vertex {
    pub fn is_nan(&self) -> bool {
        self.0.is_nan() || self.1.is_nan() || self.2.is_nan()
    }

    pub fn is_infinite(&self) -> bool {
        self.0.is_infinite() || self.1.is_infinite() || self.2.is_infinite()
    }
}

#[derive(Debug)]
pub struct Tractogram {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub fn load_tck(bytes: &[u8]) -> Tractogram {
    let offset = bytes
        .windows(4)
        .position(|w| w == b"END\n")
        .map(|ix| ix + 4)
        .expect(".tck file does not contain 'END'");

    let payload = bytes[offset..].to_owned();

    let vertices: Vec<Vec<Vertex>> = cast_slice(&payload)
        .split(|x: &Vertex| x.is_nan() || x.is_infinite())
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

    let vertices: Vec<Vertex> = vertices.into_iter().flatten().collect();

    Tractogram { vertices, indices }
}
