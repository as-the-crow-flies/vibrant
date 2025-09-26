use criterion::{criterion_group, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{
        line::LineSet,
        utils::{load_line, load_line_brain_1m, load_line_brain_200k, BenchmarkLineSet},
    },
    controller::{
        settings::{LineDisplayMode, LineRenderMode, LineVoxelizationMode, Settings},
        Controller,
    },
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::{occupancy::LineOccupancyPipeline, transform::LineTransformPipeline},
    },
    surface::Frame,
};

pub fn get_settings(volume: u32, voxelization: LineVoxelizationMode) -> Settings {
    Settings {
        // Settings that matter for this experiment
        volume,
        voxelization,
        radius: 0.2,
        alpha: 1.0,
        smoothing: 0.5,

        // Remaining Settings
        width: 1920,
        height: 1080,
        lighting: 1.0,
        direct_light: 0.5,
        tangent_color: 1.0,
        shadows: 0.0,
        level: 0.0,
        slice_count: 10,
        culling: false,
        render: LineRenderMode::RayTracing,
        display: LineDisplayMode::Geometry,
    }
}

pub fn occupancy_benchmark(
    criterion: &mut Criterion,
    gpu: &Gpu,
    set: BenchmarkLineSet,
    line: &LineSet,
    volume: u32,
    voxelization: LineVoxelizationMode,
    stride: usize,
) {
    let settings = get_settings(volume, voxelization);
    let controller = &Controller::from_settings(&settings);
    let environment = &Environment::from_controller(gpu, controller);
    let frame = &Frame::new(gpu, &settings);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).dispatch(&mut cmd, environment, line);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = LineOccupancyPipeline::new(gpu);

    let id = format!(
        "occupancy {:?} - {:?} - {:?} - {:?}",
        set, volume, voxelization, stride
    );

    criterion.bench_function(&id, |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.dispatch(&mut cmd, frame, environment, voxelization, line);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn occupancy_experiment(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    for set in BenchmarkLineSet::iter() {
        println!("Loading Line Set {:?}", set);

        let line = &load_line(gpu, set);

        for volume in [128, 256, 512] {
            for voxelization in LineVoxelizationMode::iter() {
                occupancy_benchmark(criterion, gpu, set, line, volume, voxelization, 1);
            }
        }
    }

    for stride in 1..=13 {
        let line = &load_line_brain_1m(gpu, stride);
        let volume = 256;

        for voxelization in LineVoxelizationMode::iter() {
            occupancy_benchmark(
                criterion,
                gpu,
                BenchmarkLineSet::Brain200k,
                line,
                volume,
                voxelization,
                stride,
            );
        }
    }
}

pub fn occupancy_experiment_2(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    occupancy_benchmark(
        criterion,
        gpu,
        BenchmarkLineSet::Brain200k,
        &load_line_brain_200k(gpu, 1),
        256,
        LineVoxelizationMode::Tube,
        1,
    );
}

criterion_group!(occupancy, occupancy_experiment_2);
