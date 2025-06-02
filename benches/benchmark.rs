use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::tractogram::Tractogram,
    controller::Controller,
    file::TractogramFile,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        tractogram::{density::TractogramDensityPipeline, occlusion::TractogramOcclusionPipeline},
    },
    surface::SurfaceBuffer,
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const VOLUME: u32 = 256;
const TILE: u32 = 4;

const LAYERS: u32 = 32;

const TRACTOGRAM_PATH: &'static str = "assets/HCP-100307/whole_brain200k.tck";

pub fn get_controller() -> Controller {
    Controller::test(WIDTH, HEIGHT, VOLUME, TILE, LAYERS)
}

pub fn get_environment(gpu: &Gpu) -> Environment {
    let environment = Environment::new(&gpu);
    environment.update(&gpu, &get_controller());
    return environment;
}

pub fn empty(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    criterion.bench_function("empty", |bencher| {
        bencher.iter(|| {
            let cmd = gpu.cmd();
            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn density(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &SurfaceBuffer::new(gpu, &get_controller());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let pipeline = TractogramDensityPipeline::new(gpu);

    criterion.bench_function("density", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn occlusion(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &SurfaceBuffer::new(gpu, &get_controller());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let mut cmd = gpu.cmd();
    TractogramDensityPipeline::new(gpu).render(&mut cmd, frame, environment, tractogram);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = TractogramOcclusionPipeline::new(gpu);

    criterion.bench_function("occlusion", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, empty, density, occlusion);
criterion_main!(benches);
