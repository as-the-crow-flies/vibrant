pub mod abuffer;
pub mod occupancy;
pub mod render;

use criterion::criterion_main;

criterion_main!(occupancy::occupancy, abuffer::abuffer, render::render);
