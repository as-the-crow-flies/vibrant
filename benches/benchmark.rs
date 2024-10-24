use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{density::Density, tractogram::Tractogram},
    controller::{event::Event, Controller},
    gpu::Gpu,
    loader,
    renderer::{
        constants::Constants,
        environment::Environment,
        services::filter::{Filter, FilterDescriptor},
        services::scan::{ItemType, Scan, ScanDescriptor},
        tractogram::density::compute::TractogramDensityComputeRenderer,
        tractogram::full::render::TractogramRenderer,
        tractogram::line::compute::TractogramLineComputeRenderer,
        tractogram::line::render::TractogramLineRenderRenderer,
    },
    surface::Frame,
    Vec2,
};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages,
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const VOLUME: u32 = 9;

const TRACTOGRAM_PATH: &'static str = "assets/HPC-100307/whole_brain1M.tck";

pub fn get_environment(gpu: &Gpu) -> Environment {
    let environment = Environment::new(&gpu);
    let mut controller = Controller::new();
    controller.event(Event::Resized(Vec2::new(WIDTH as f32, HEIGHT as f32)));
    environment.update(&gpu, &controller);
    return environment;
}

pub fn baseline(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let environment = get_environment(&gpu);

    let tractogram = Tractogram::new(&gpu, &loader::Tck::from_file(TRACTOGRAM_PATH));

    let renderer = TractogramLineRenderRenderer::new(&gpu);

    criterion.bench_function(stringify!(baseline), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            let frame = Frame::test(&gpu, WIDTH, HEIGHT);

            renderer.render(&mut cmd, &environment, &frame, &tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn compute(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let environment = get_environment(&gpu);

    let tractogram = Tractogram::new(&gpu, &loader::Tck::from_file(TRACTOGRAM_PATH));

    let constants = Constants::new(&gpu, (WIDTH, HEIGHT), 9);
    let renderer = TractogramLineComputeRenderer::new(&gpu, &constants);

    criterion.bench_function(stringify!(compute), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            let frame = Frame::test(&gpu, WIDTH, HEIGHT);

            renderer.render(&mut cmd, &environment, &frame, &tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn density(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let environment = get_environment(&gpu);

    let tractogram = Tractogram::new(&gpu, &loader::Tck::from_file(TRACTOGRAM_PATH));

    let density = Density::new(&gpu, VOLUME);
    let constants = Constants::new(&gpu, (WIDTH, HEIGHT), density.size());

    let renderer = TractogramDensityComputeRenderer::new(&gpu, &constants);

    criterion.bench_function(stringify!(density), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            renderer.render(&mut cmd, &environment, &tractogram, &density);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn render(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let environment = get_environment(&gpu);

    let tractogram = Tractogram::new(&gpu, &loader::Tck::from_file(TRACTOGRAM_PATH));

    let density = Density::new(&gpu, VOLUME);
    let constants = Constants::new(&gpu, (WIDTH, HEIGHT), density.size());

    {
        let mut cmd = gpu.cmd();
        TractogramDensityComputeRenderer::new(&gpu, &constants).render(
            &mut cmd,
            &environment,
            &tractogram,
            &density,
        );

        gpu.submit(cmd);
        gpu.wait();
    }

    let renderer = TractogramRenderer::new(&gpu, &constants);

    criterion.bench_function(stringify!(render), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            let frame = Frame::test(&gpu, WIDTH, HEIGHT);

            renderer.render(&mut cmd, &environment, &frame, &tractogram, &density);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

const DATA_SIZE: u32 = 64 * 1024 * 1024;

pub fn scan(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    let array: Vec<u32> = (0..DATA_SIZE).into_iter().map(|_| 1u32).collect();

    let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&array),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    });

    let scan = Scan::new(
        &gpu,
        &ScanDescriptor {
            scan: &buffer,
            item_type: ItemType::U32,
            items_per_thread: 16,
        },
    );

    criterion.bench_function(stringify!(scan), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            scan.compute(&mut cmd);
            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn filter(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    let array: Vec<u32> = (0..DATA_SIZE).into_iter().map(|_| 1u32).collect();

    let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&array),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    });

    let filter = Filter::new(
        &gpu,
        &FilterDescriptor {
            buffer: &buffer,
            workgroup_size: 256,
            items_per_thread: 16,
        },
    );

    criterion.bench_function(stringify!(filter), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            filter.compute(&mut cmd);
            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, baseline, compute, density, render, scan, filter);
criterion_main!(benches);
