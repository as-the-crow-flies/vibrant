use std::f32::consts::PI;

use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::tractogram::Tractogram,
    controller::{self, camera::Camera, light::Light, Controller},
    gpu::Gpu,
    loader,
    renderer::{environment::Environment, tractogram_baseline::BaselineTractogramRenderer},
    surface::Frame,
};

pub fn baseline_benchmark(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let environment = Environment::new(&gpu);
    environment.update(&gpu, &Controller::new());

    let tractogram = Tractogram::new(
        &gpu,
        &loader::Tractogram::from_file("assets/whole_brain1M.tck"),
    );

    let renderer = BaselineTractogramRenderer::new(&gpu);

    criterion.bench_function("baseline render", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            let frame = Frame::test(&gpu, 1024, 1024);

            renderer.render(&mut cmd, &environment, &frame, &tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, baseline_benchmark);
criterion_main!(benches);
