use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{density::Density, tractogram::Tractogram},
    controller::{event::Event, Controller},
    file,
    gpu::Gpu,
    renderer::{
        constants::Constants,
        environment::Environment,
        tractogram::{
            density::TractogramDensityComputeRenderer, line::render::TractogramLineGeometry,
            shading::tracing::TractogramTracingShading,
        },
    },
    surface::{buffer::FrameBuffer, depth::Depth, gbuffer::GBuffer, visibility::Visibility, Frame},
    Vec2,
};
use wgpu::Texture;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const VOLUME: u32 = 9;

const TRACTOGRAM_PATH: &'static str = "assets/HPC-100307/whole_brain1M.tck";

struct TestSurface {
    buffer: Texture,
    visibility: Visibility,
    depth: Depth,
    gbuffer: GBuffer,
}

impl TestSurface {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            buffer: FrameBuffer::texture(gpu, WIDTH, HEIGHT),
            visibility: Visibility::new(gpu, WIDTH, HEIGHT),
            depth: Depth::new(gpu, WIDTH, HEIGHT),
            gbuffer: GBuffer::new(gpu, WIDTH, HEIGHT),
        }
    }

    pub fn frame<'a>(&'a self) -> Frame<'a> {
        Frame {
            width: WIDTH,
            height: HEIGHT,
            buffer: FrameBuffer::new(&self.buffer),
            visibility: &self.visibility,
            depth: &self.depth,
            gbuffer: &self.gbuffer,
        }
    }
}

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

    let tractogram = Tractogram::new(&gpu, &file::Tck::from_file(TRACTOGRAM_PATH));

    let renderer = TractogramLineGeometry::new(&gpu);

    criterion.bench_function(stringify!(baseline), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            let surface = TestSurface::new(&gpu);
            let frame = surface.frame();

            renderer.render(
                &mut cmd,
                &environment,
                &frame,
                &tractogram,
                &tractogram.filter_default(),
            );

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn density(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let environment = get_environment(&gpu);

    let tractogram = Tractogram::new(&gpu, &file::Tck::from_file(TRACTOGRAM_PATH));

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

    let tractogram = Tractogram::new(&gpu, &file::Tck::from_file(TRACTOGRAM_PATH));

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

    let renderer = TractogramTracingShading::new(&gpu, &constants);

    criterion.bench_function(stringify!(render), |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            let surface = TestSurface::new(&gpu);
            let frame = surface.frame();

            renderer.render(&mut cmd, &environment, &frame, &density, &tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, baseline, density, render);
criterion_main!(benches);
