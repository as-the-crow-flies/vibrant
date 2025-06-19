use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::tractogram::Tractogram,
    controller::{settings::Settings, Controller},
    file::TractogramFile,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        tractogram::{
            adjacency::AdjacenyPipeline, density::DensityPipeline, occlusion::OcclusionPipeline,
            occupancy::OccupancyPipeline, populate::PopulatePipeline,
            render::TractogramRenderPipeline,
        },
    },
    surface::Frame,
};

const TRACTOGRAM_PATH: &'static str = "assets/HCP-100307/whole_brain200k.tck";

pub fn get_environment(gpu: &Gpu) -> Environment {
    let environment = Environment::new(&gpu);
    environment.update(&gpu, &Controller::new());
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
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let mut cmd = gpu.cmd();
    AdjacenyPipeline::new(gpu).render(&mut cmd, tractogram);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = DensityPipeline::new(gpu);

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
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let mut cmd = gpu.cmd();
    AdjacenyPipeline::new(gpu).render(&mut cmd, tractogram);
    DensityPipeline::new(gpu).render(&mut cmd, frame, environment, tractogram);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = OcclusionPipeline::new(gpu);

    criterion.bench_function("occlusion", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn occupancy(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let mut cmd = gpu.cmd();
    AdjacenyPipeline::new(gpu).render(&mut cmd, tractogram);
    DensityPipeline::new(gpu).render(&mut cmd, frame, environment, tractogram);
    OcclusionPipeline::new(gpu).render(&mut cmd, frame, environment);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = OccupancyPipeline::new(gpu);

    criterion.bench_function("occupancy", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn populate(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let mut cmd = gpu.cmd();
    AdjacenyPipeline::new(gpu).render(&mut cmd, tractogram);
    DensityPipeline::new(gpu).render(&mut cmd, frame, environment, tractogram);
    OcclusionPipeline::new(gpu).render(&mut cmd, frame, environment);
    gpu.submit(cmd);
    gpu.wait();

    let occupancy = OccupancyPipeline::new(gpu);
    let populate = PopulatePipeline::new(gpu);

    criterion.bench_function("populate", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            occupancy.render(&mut cmd, frame, environment);
            populate.render(&mut cmd, frame, environment, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn render(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let mut cmd = gpu.cmd();
    AdjacenyPipeline::new(gpu).render(&mut cmd, tractogram);
    DensityPipeline::new(gpu).render(&mut cmd, frame, environment, tractogram);
    OcclusionPipeline::new(gpu).render(&mut cmd, frame, environment);
    OccupancyPipeline::new(gpu).render(&mut cmd, frame, environment);
    PopulatePipeline::new(gpu).render(&mut cmd, frame, environment, tractogram);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = TractogramRenderPipeline::new(gpu);

    criterion.bench_function("render", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn full(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &Tractogram::new(gpu, &TractogramFile::from_file(TRACTOGRAM_PATH));

    let mut cmd = gpu.cmd();
    AdjacenyPipeline::new(gpu).render(&mut cmd, tractogram);
    gpu.submit(cmd);
    gpu.wait();

    let density = DensityPipeline::new(gpu);
    let occlusion = OcclusionPipeline::new(gpu);
    let occupancy = OccupancyPipeline::new(gpu);
    let populate = PopulatePipeline::new(gpu);
    let render = TractogramRenderPipeline::new(gpu);

    criterion.bench_function("full", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            density.render(&mut cmd, frame, environment, tractogram);
            occlusion.render(&mut cmd, frame, environment);
            occupancy.render(&mut cmd, frame, environment);
            populate.render(&mut cmd, frame, environment, tractogram);
            render.render(&mut cmd, frame, environment, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, empty, density, occlusion, occupancy, populate, render, full);
criterion_main!(benches);
