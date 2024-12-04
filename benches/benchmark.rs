use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{density::Density, filter::Filter, occlusion::Occlusion, tractogram::Tractogram},
    controller::{event::Event, Controller},
    file::Tck,
    gpu::Gpu,
    renderer::{
        constants::Constants,
        environment::Environment,
        tractogram::{
            density::add::TractogramDensityAddCompute,
            geometry::{line::TractogramLineGeometry, tube::TractogramTubeGeometry},
            occlusion::TractogramOcclusionCompute,
            shading::{simple::TractogramSimpleShading, tracing::TractogramTracingShading},
        },
    },
    surface::{buffer::FrameBuffer, depth::Depth, gbuffer::GBuffer, Frame},
    Vec2,
};
use wgpu::{CommandEncoder, Texture};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const VOLUME: u32 = 7;

struct TestSurface {
    buffer: Texture,
    depth: Depth,
    gbuffer: GBuffer,
}

impl TestSurface {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            buffer: FrameBuffer::texture(gpu, WIDTH, HEIGHT),
            depth: Depth::new(gpu, WIDTH, HEIGHT),
            gbuffer: GBuffer::new(gpu, WIDTH, HEIGHT),
        }
    }

    pub fn frame<'a>(&'a self) -> Frame<'a> {
        Frame {
            width: WIDTH,
            height: HEIGHT,
            buffer: FrameBuffer::new(&self.buffer),
            depth: &self.depth,
            gbuffer: &self.gbuffer,
        }
    }
}

pub trait TractogramGeometry {
    fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        filter: &Filter,
    );
}

impl TractogramGeometry for TractogramLineGeometry {
    fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        TractogramLineGeometry::render(&self, cmd, environment, frame, tractogram, filter);
    }
}

impl TractogramGeometry for TractogramTubeGeometry {
    fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        TractogramTubeGeometry::render(&self, cmd, environment, frame, tractogram, filter);
    }
}

pub fn get_environment(gpu: &Gpu) -> Environment {
    let environment = Environment::new(&gpu);
    let mut controller = Controller::new();
    controller.event(Event::Resized(Vec2::new(WIDTH as f32, HEIGHT as f32)));
    environment.update(&gpu, &controller);
    return environment;
}

pub fn get_whole_brain_tractogram_200k(gpu: &Gpu) -> Tractogram {
    Tractogram::new(
        gpu,
        &Tck::from_file("assets/HCP-100307/whole_brain200k.tck"),
    )
}

pub fn get_whole_brain_tractogram_1m(gpu: &Gpu) -> Tractogram {
    Tractogram::new(gpu, &Tck::from_file("assets/HCP-100307/whole_brain1M.tck"))
}

pub fn get_cst_tractogram(gpu: &Gpu) -> Tractogram {
    Tractogram::new(
        gpu,
        &Tck::join(vec![
            Tck::from_file("assets/HCP-100307/TOM_trackings/AF_left.tck"),
            Tck::from_file("assets/HCP-100307/TOM_trackings/AF_right.tck"),
            Tck::from_file("assets/HCP-100307/TOM_trackings/ATR_left.tck"),
            Tck::from_file("assets/HCP-100307/TOM_trackings/ATR_right.tck"),
            Tck::from_file("assets/HCP-100307/TOM_trackings/CST_left.tck"),
            Tck::from_file("assets/HCP-100307/TOM_trackings/CST_right.tck"),
            Tck::from_file("assets/HCP-100307/TOM_trackings/FPT_left.tck"),
            Tck::from_file("assets/HCP-100307/TOM_trackings/FPT_right.tck"),
        ]),
    )
}

pub fn baseline_line_brain_200k(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    baseline(
        criterion,
        stringify!(baseline_line_brain_200k),
        &gpu,
        &get_whole_brain_tractogram_200k(&gpu),
        &TractogramLineGeometry::new(&gpu),
    );
}

pub fn baseline_line_brain_1m(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    baseline(
        criterion,
        stringify!(baseline_line_brain_1m),
        &gpu,
        &get_whole_brain_tractogram_1m(&gpu),
        &TractogramLineGeometry::new(&gpu),
    );
}

pub fn baseline_line_cst(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    baseline(
        criterion,
        stringify!(baseline_line_cst),
        &gpu,
        &get_cst_tractogram(&gpu),
        &TractogramLineGeometry::new(&gpu),
    );
}

pub fn baseline_tube_brain_200k(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    baseline(
        criterion,
        stringify!(baseline_tube_brain_200k),
        &gpu,
        &get_whole_brain_tractogram_200k(&gpu),
        &TractogramTubeGeometry::new(&gpu),
    );
}

pub fn baseline_tube_brain_1m(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    baseline(
        criterion,
        stringify!(baseline_tube_brain_1m),
        &gpu,
        &get_whole_brain_tractogram_1m(&gpu),
        &TractogramTubeGeometry::new(&gpu),
    );
}

pub fn baseline_tube_cst(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    baseline(
        criterion,
        stringify!(baseline_tube_cst),
        &gpu,
        &get_cst_tractogram(&gpu),
        &TractogramTubeGeometry::new(&gpu),
    );
}

pub fn shading_line_brain_200k(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    shading(
        criterion,
        stringify!(shading_line_brain_200k),
        &gpu,
        &get_whole_brain_tractogram_200k(&gpu),
        &TractogramLineGeometry::new(&gpu),
    );
}

pub fn shading_line_brain_1m(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    shading(
        criterion,
        stringify!(shading_line_brain_1m),
        &gpu,
        &get_whole_brain_tractogram_1m(&gpu),
        &TractogramLineGeometry::new(&gpu),
    );
}

pub fn shading_line_cst(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    shading(
        criterion,
        stringify!(shading_line_cst),
        &gpu,
        &get_cst_tractogram(&gpu),
        &TractogramLineGeometry::new(&gpu),
    );
}

pub fn shading_tube_brain_200k(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    shading(
        criterion,
        stringify!(shading_tube_brain_200k),
        &gpu,
        &get_whole_brain_tractogram_200k(&gpu),
        &TractogramTubeGeometry::new(&gpu),
    );
}

pub fn shading_tube_brain_1m(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    shading(
        criterion,
        stringify!(shading_tube_brain_1m),
        &gpu,
        &get_whole_brain_tractogram_1m(&gpu),
        &TractogramTubeGeometry::new(&gpu),
    );
}

pub fn shading_tube_cst(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();
    shading(
        criterion,
        stringify!(shading_tube_cst),
        &gpu,
        &get_cst_tractogram(&gpu),
        &TractogramTubeGeometry::new(&gpu),
    );
}

pub fn baseline(
    criterion: &mut Criterion,
    id: &str,
    gpu: &Gpu,
    tractogram: &Tractogram,
    geometry: &impl TractogramGeometry,
) {
    let environment = get_environment(gpu);
    let surface = TestSurface::new(gpu);
    let simple_shading = TractogramSimpleShading::new(gpu);

    criterion.bench_function(id, |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            let frame = surface.frame();

            geometry.render(
                &mut cmd,
                &environment,
                &frame,
                &tractogram,
                &tractogram.filter_default(),
            );

            simple_shading.render(&mut cmd, &frame, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        });
    });

    gpu.save(
        format!("target/criterion/{}.png", id).into(),
        &surface.buffer,
    )
    .block_on();
}

pub fn shading(
    criterion: &mut Criterion,
    id: &str,
    gpu: &Gpu,
    tractogram: &Tractogram,
    geometry: &impl TractogramGeometry,
) {
    let environment = get_environment(&gpu);
    let surface = TestSurface::new(gpu);

    let density = Density::new(&gpu, VOLUME);
    let occlusion = Occlusion::new(&gpu, VOLUME);
    let constants = Constants::new(&gpu, (WIDTH, HEIGHT), density.size());

    let density_compute = TractogramDensityAddCompute::new(&gpu, &constants);
    let occlusion_compute = TractogramOcclusionCompute::new(gpu, &constants);
    let tracing_shading = TractogramTracingShading::new(gpu, &constants);

    criterion.bench_function(id, |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            let frame = surface.frame();

            density_compute.render(&mut cmd, &environment, tractogram, &density);
            occlusion_compute.render(&mut cmd, &environment, &density, &occlusion, tractogram);

            geometry.render(
                &mut cmd,
                &environment,
                &frame,
                &tractogram,
                &tractogram.filter_culling(),
            );

            tracing_shading.render(&mut cmd, &environment, &frame, &density, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        });
    });

    gpu.save(
        format!("target/criterion/{}.png", id).into(),
        &surface.buffer,
    )
    .block_on();
}

criterion_group!(
    benches,
    baseline_line_brain_200k,
    baseline_line_brain_1m,
    baseline_line_cst,
    baseline_tube_brain_200k,
    baseline_tube_brain_1m,
    baseline_tube_cst,
    shading_line_brain_200k,
    shading_line_brain_1m,
    shading_line_cst,
    shading_tube_brain_200k,
    shading_tube_brain_1m,
    shading_tube_cst,
);
criterion_main!(benches);
