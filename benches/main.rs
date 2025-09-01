pub mod abuffer;
pub mod occupancy;

use criterion::criterion_main;

criterion_main!(occupancy::occupancy, abuffer::abuffer);
