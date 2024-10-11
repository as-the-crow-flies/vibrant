use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{density::Density, tractogram::Tractogram},
    controller::{event::Event, Controller},
    gpu::Gpu,
    loader,
    renderer::{
        constants::Constants,
        environment::{self, Environment},
        tractogram_baseline::BaselineTractogramRenderer,
        tractogram_density::TractogramDensityRenderer,
        tractogram_render::TractogramRenderer,
    },
    surface::Frame,
    Vec2,
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const VOLUME: u32 = 9;

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

    let tractogram = Tractogram::new(
        &gpu,
        &loader::Tractogram::from_file("assets/whole_brain1M.tck"),
    );

    let renderer = BaselineTractogramRenderer::new(&gpu);

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

pub fn density(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let environment = get_environment(&gpu);

    let tractogram = Tractogram::new(
        &gpu,
        &loader::Tractogram::from_file("assets/whole_brain1M.tck"),
    );

    let density = Density::new(&gpu, VOLUME);
    let constants = Constants::new(&gpu, (WIDTH, HEIGHT), density.size());

    let renderer = TractogramDensityRenderer::new(&gpu, &constants);

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

    let tractogram = Tractogram::new(
        &gpu,
        &loader::Tractogram::from_file("assets/whole_brain1M.tck"),
    );

    let density = Density::new(&gpu, VOLUME);
    let constants = Constants::new(&gpu, (WIDTH, HEIGHT), density.size());

    {
        let mut cmd = gpu.cmd();
        TractogramDensityRenderer::new(&gpu, &constants).render(
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

criterion_group!(benches, baseline, density, render);
criterion_main!(benches);
