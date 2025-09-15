use criterion::{criterion_group, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{
        line::LineSet,
        utils::{load_line, BenchmarkLineSet},
    },
    controller::{
        settings::{LineDisplayMode, LineRenderMode, LineVoxelizationMode, Settings},
        Controller,
    },
    gpu::Gpu,
    renderer::{environment::Environment, line::LineRenderer},
    surface::Frame,
};

fn get_settings(
    render: LineRenderMode,
    voxelization: LineVoxelizationMode,
    alpha: f32,
    shadows: f32,
) -> Settings {
    Settings {
        render,
        voxelization,
        alpha,
        shadows,

        display: LineDisplayMode::Geometry,
        width: 1920,
        height: 1080,
        volume: 128,
        radius: 0.2,
        lighting: 1.0,
        direct_light: 0.67,
        tangent_color: 1.0,
        level: 0.0,
        smoothing: 0.5,
        culling: true,
        slice_count: 10,
    }
}

pub fn render_benchmark(
    criterion: &mut Criterion,
    gpu: &Gpu,
    settings: &Settings,
    set: BenchmarkLineSet,
    line: &LineSet,
) {
    let controller = &Controller::from_settings(settings);
    let environment = &Environment::from_controller(gpu, controller);
    let frame = &Frame::new(gpu, &settings);

    let renderer = LineRenderer::new(gpu);

    criterion.bench_function(
        &format!(
            "render - {:?} - {:?} - {:?} - {:?}",
            set, settings.render, settings.alpha, settings.shadows
        ),
        |bencher| {
            bencher.iter(|| {
                let mut cmd = gpu.cmd();

                renderer.render(&mut cmd, environment, frame, line, settings);

                gpu.submit(cmd);
                gpu.wait();
            })
        },
    );
}

pub fn render_experiment(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    for set in BenchmarkLineSet::iter() {
        println!("Loading Line Set {:?}", set);

        let line = &load_line(gpu, set);

        for alpha in [1.0, 0.1] {
            render_benchmark(
                criterion,
                gpu,
                &get_settings(
                    LineRenderMode::Rasterization,
                    LineVoxelizationMode::Line,
                    alpha,
                    0.0,
                ),
                set,
                line,
            );
            render_benchmark(
                criterion,
                gpu,
                &get_settings(
                    LineRenderMode::RasterizationOrderCorrecting,
                    LineVoxelizationMode::Line,
                    alpha,
                    0.0,
                ),
                set,
                line,
            );
            render_benchmark(
                criterion,
                gpu,
                &get_settings(
                    LineRenderMode::RayTracingQuantized,
                    LineVoxelizationMode::Line,
                    alpha,
                    0.0,
                ),
                set,
                line,
            );
            render_benchmark(
                criterion,
                gpu,
                &get_settings(
                    LineRenderMode::RayTracingQuantized,
                    LineVoxelizationMode::Line,
                    alpha,
                    1.0,
                ),
                set,
                line,
            );
            render_benchmark(
                criterion,
                gpu,
                &get_settings(
                    LineRenderMode::RayTracing,
                    LineVoxelizationMode::Tube,
                    alpha,
                    0.0,
                ),
                set,
                line,
            );
        }
    }
}

pub fn render_experiment_alpha(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let set = BenchmarkLineSet::Aneurysm;
    let line = &load_line(gpu, set);

    for alpha_step in 0..=20 {
        let alpha = ((alpha_step as f32) * 0.05 + 0.01).min(1.0);

        render_benchmark(
            criterion,
            gpu,
            &get_settings(
                LineRenderMode::RasterizationOrderCorrecting,
                LineVoxelizationMode::Line,
                alpha,
                0.0,
            ),
            set,
            line,
        );
        render_benchmark(
            criterion,
            gpu,
            &get_settings(
                LineRenderMode::RayTracingQuantized,
                LineVoxelizationMode::Line,
                alpha,
                0.0,
            ),
            set,
            line,
        );
        render_benchmark(
            criterion,
            gpu,
            &get_settings(
                LineRenderMode::RayTracing,
                LineVoxelizationMode::Tube,
                alpha,
                0.0,
            ),
            set,
            line,
        );
    }
}

criterion_group!(render, render_experiment);
