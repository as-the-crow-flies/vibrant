use criterion::{criterion_group, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{
        line::LineSet,
        utils::{load_line, BenchmarkLineSet},
    },
    bool,
    controller::{
        settings::{LineDisplayMode, LineRenderMode, LineVoxelizationMode, Settings},
        Controller,
    },
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::{
            culling::LineCullingPipeline, occupancy::LineOccupancyPipeline,
            occupancy_alt::LineOccupancyAltPipeline,
            render::raytracing::populate::LinePopulatePipeline, transform::LineTransformPipeline,
            vrc::VrcLineVoxelizationPipeline,
        },
    },
    surface::Frame,
};

pub fn get_settings(volume: u32, voxelization: LineVoxelizationMode, culling: bool) -> Settings {
    Settings {
        // Settings that matter for this experiment
        volume,
        culling,
        voxelization,
        radius: 0.75,
        alpha: 1.0,
        smoothing: 0.5,
        workgroups: 18,

        // Remaining Settings
        width: 1920,
        height: 1080,
        lighting: 1.0,
        direct_light: 0.5,
        tangent_color: 1.0,
        shadows: 1.0,
        level: 0.0,
        slice_count: 10,
        render: LineRenderMode::RayTracing,
        display: LineDisplayMode::Geometry,
    }
}

pub fn abuffer_benchmark_ours(
    criterion: &mut Criterion,
    gpu: &Gpu,
    set: BenchmarkLineSet,
    line: &LineSet,
    volume: u32,
    voxelization: LineVoxelizationMode,
    culling: bool,
) {
    let settings = get_settings(volume, voxelization, culling);
    let controller = &Controller::from_settings(&settings);
    let environment = &Environment::from_controller(gpu, controller);
    let frame = &Frame::new(gpu, &settings);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).dispatch(&mut cmd, environment, line);
    gpu.submit(cmd);
    gpu.wait();

    let occupancy = LineOccupancyPipeline::new(gpu);
    let cull = LineCullingPipeline::new(gpu);
    let populate = LinePopulatePipeline::new(gpu);

    criterion.bench_function(
        &format!(
            "abuffer - ours - {:?} - {:?} - {:?} - culling: {:?}",
            set, volume, voxelization, culling
        ),
        |bencher| {
            bencher.iter(|| {
                let mut cmd = gpu.cmd();

                occupancy.dispatch(&mut cmd, frame, environment, &settings, line);
                cull.dispatch(&mut cmd, frame, environment);
                populate.dispatch(&mut cmd, frame, environment, &settings, line);

                gpu.submit(cmd);
                gpu.wait();
            })
        },
    );
}

pub fn abuffer_benchmark_alt(
    criterion: &mut Criterion,
    gpu: &Gpu,
    set: BenchmarkLineSet,
    line: &LineSet,
    volume: u32,
    voxelization: LineVoxelizationMode,
) {
    let settings = get_settings(volume, voxelization, false);
    let controller = &Controller::from_settings(&settings);
    let environment = &Environment::from_controller(gpu, controller);
    let frame = &Frame::new(gpu, &settings);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).dispatch(&mut cmd, environment, line);
    gpu.submit(cmd);
    gpu.wait();

    let occpancy_alt = LineOccupancyAltPipeline::new(gpu);

    criterion.bench_function(
        &format!(
            "abuffer - alt - {:?} - {:?} - {:?}",
            set, volume, voxelization
        ),
        |bencher| {
            bencher.iter(|| {
                let mut cmd = gpu.cmd();

                occpancy_alt.dispatch(&mut cmd, frame, environment, &settings, line);

                gpu.submit(cmd);
                gpu.wait();
            })
        },
    );
}

pub fn abuffer_benchmark_vrc(
    criterion: &mut Criterion,
    gpu: &Gpu,
    set: BenchmarkLineSet,
    line: &LineSet,
    volume: u32,
) {
    let settings = get_settings(volume, LineVoxelizationMode::Line, true);
    let controller = &Controller::from_settings(&settings);
    let environment = &Environment::from_controller(gpu, controller);
    let frame = &Frame::new(gpu, &Settings::new());

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).dispatch(&mut cmd, environment, line);
    gpu.submit(cmd);
    gpu.wait();

    let vrc = VrcLineVoxelizationPipeline::new(gpu);

    criterion.bench_function(
        &format!("abuffer - vrc - {:?} - {:?}", set, volume),
        |bencher| {
            bencher.iter(|| {
                let mut cmd = gpu.cmd();

                vrc.dispatch(&mut cmd, frame, environment, &settings, line);

                gpu.submit(cmd);
                gpu.wait();
            })
        },
    );
}

pub fn abuffer_experiment(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    for set in BenchmarkLineSet::iter() {
        println!("Loading Line Set {:?}", set);

        let line = &load_line(gpu, set);

        for volume in [128] {
            abuffer_benchmark_vrc(criterion, gpu, set, line, volume);
            abuffer_benchmark_alt(
                criterion,
                gpu,
                set,
                line,
                volume,
                LineVoxelizationMode::Tube,
            );
            abuffer_benchmark_ours(
                criterion,
                gpu,
                set,
                line,
                volume,
                LineVoxelizationMode::Tube,
                false,
            );
            abuffer_benchmark_ours(
                criterion,
                gpu,
                set,
                line,
                volume,
                LineVoxelizationMode::Tube,
                true,
            );
        }
    }
}

criterion_group!(abuffer, abuffer_experiment);
