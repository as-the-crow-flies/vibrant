use std::fs;

use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    controller::Controller,
    file::{File, LineFile},
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::{
            crop::LineCropPipeline, occupancy::LineOccupancyPipeline,
            transform::LineTransformPipeline,
        },
    },
    surface::Frame,
};

fn occlusion(criterion: &mut Criterion, id: &str, gpu: &Gpu, line: &LineBuffer) {
    criterion.bench_function(id, |bench| {
        let controller = &Controller::new();
        let environment = &Environment::from_controller(gpu, controller);
        let transform = &TransformBuffer::new(gpu, line.bounds().transform().inverse());
        let frame = &Frame::new(gpu, controller.settings());

        let mut cmd = gpu.cmd();
        LineTransformPipeline::new(gpu).dispatch(&mut cmd, line, transform);
        LineCropPipeline::new(gpu).dispatch(&mut cmd, line, environment);
        gpu.submit(cmd);
        gpu.wait();

        let pipeline = LineOccupancyPipeline::new(gpu);

        bench.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.dispatch(&mut cmd, frame, environment, controller.settings(), line);

            gpu.submit(cmd);
            gpu.wait();
        });
    });
}

fn occlusion_experiment(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let id = "Brain200k";

    let file = LineFile::from_tck(&File::new(
        id,
        fs::read("assets/HCP-100307/whole_brain200k.tck").unwrap(),
    ));

    let buffer = LineBuffer::new(gpu, &[file]);

    occlusion(criterion, id, gpu, &buffer);
}

criterion_group!(benches, occlusion_experiment);
criterion_main!(benches);
