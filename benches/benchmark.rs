use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::tractogram::Tractogram,
    controller::Controller,
    file::Tck,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        tractogram::{
            density::TractogramDensityPipeline,
            geometry::{line::TractogramLineGeometry, tube::TractogramTubeGeometry},
            occlusion::TractogramOcclusionPipeline,
        },
    },
    surface::{density::Density, SurfaceBuffer},
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const VOLUME: u32 = 256;
const TILE: u32 = 8;

const TRACTOGRAM_PATH: &'static str = "assets/HCP-100307/whole_brain200k.tck";

pub fn get_environment(gpu: &Gpu) -> Environment {
    let environment = Environment::new(&gpu);
    let controller = Controller::test(WIDTH, HEIGHT, VOLUME, TILE);
    environment.update(&gpu, &controller);
    return environment;
}

pub fn line(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &SurfaceBuffer::new(gpu, WIDTH, HEIGHT, VOLUME, 8);
    let tractogram = &Tractogram::new(gpu, &Tck::from_file(TRACTOGRAM_PATH));

    let pipeline = TractogramLineGeometry::new(gpu);

    criterion.bench_function("line", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(
                &mut cmd,
                environment,
                frame,
                tractogram,
                tractogram.filter_default(),
            );

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn tube(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &SurfaceBuffer::new(gpu, WIDTH, HEIGHT, VOLUME, 8);
    let tractogram = &Tractogram::new(gpu, &Tck::from_file(TRACTOGRAM_PATH));

    let pipeline = TractogramTubeGeometry::new(gpu);

    criterion.bench_function("tube", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(
                &mut cmd,
                environment,
                frame,
                tractogram,
                tractogram.filter_default(),
            );

            gpu.submit(cmd);
            gpu.wait();
        })
    });
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

pub fn occlusion(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &SurfaceBuffer::new(gpu, WIDTH, HEIGHT, VOLUME, 8);
    let tractogram = &Tractogram::new(gpu, &Tck::from_file(TRACTOGRAM_PATH));

    let density = &Density::new(gpu, VOLUME);

    let mut cmd = gpu.cmd();
    TractogramDensityPipeline::new(gpu).render(&mut cmd, environment, tractogram, density);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = TractogramOcclusionPipeline::new(gpu);

    criterion.bench_function("occlusion", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, density, occlusion, line, tube);
criterion_main!(benches);
