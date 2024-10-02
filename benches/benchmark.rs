use std::{cell::RefCell, f32::consts::PI, rc::Rc};

use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    buffer::AssetBuffer,
    gpu::Gpu,
    loader::Tractogram,
    renderer::{camera::Camera, tractogram::TractogramRenderer},
    surface::TestFrame,
    Mat4, Vec3,
};

pub fn baseline_benchmark(criterion: &mut Criterion) {
    let gpu = Gpu::new().block_on();

    let camera = Camera::new(&gpu);
    camera.update(&gpu, get_mvp());

    let tractogram = Tractogram::from_file("assets/whole_brain1M.tck");
    let tractogram = AssetBuffer::new_tractogram(&gpu, &tractogram);

    let renderer = TractogramRenderer::new(&gpu, &camera, tractogram);

    criterion.bench_function("baseline render", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();
            let frame = TestFrame::new(&gpu, 1024, 1024);

            renderer.render(&mut cmd, &camera, &frame.view());

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn get_mvp() -> Mat4 {
    Mat4::perspective_lh(PI / 4.0, 1.0, 1.0, 1000.0) * Mat4::from_translation(Vec3::Z * 200.0)
}

criterion_group!(benches, baseline_benchmark);
criterion_main!(benches);
