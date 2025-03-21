use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::tractogram::Tractogram,
    controller::{event::Event, Controller},
    file::Tck,
    gpu::Gpu,
    renderer::{environment::Environment, tractogram::density::TractogramDensityPipeline},
    surface::density::Density,
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const VOLUME: u32 = 256;

const TRACTOGRAM_PATH: &'static str = "assets/HCP-100307/whole_brain200k.tck";

pub fn get_environment(gpu: &Gpu) -> Environment {
    let environment = Environment::new(&gpu);
    let mut controller = Controller::new();
    controller.event(Event::Resized(WIDTH, HEIGHT));
    environment.update(&gpu, &controller);
    return environment;
}

pub fn density(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let tractogram = &Tractogram::new(gpu, &Tck::from_file(TRACTOGRAM_PATH));

    let density = &Density::new(gpu, VOLUME);

    let pipeline = TractogramDensityPipeline::new(gpu);

    criterion.bench_function("density", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, environment, tractogram, density);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, density);
criterion_main!(benches);
