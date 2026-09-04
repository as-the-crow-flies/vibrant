// Times every GPU pass `Renderer::render` dispatches per frame, in render order,
// against one real scene (see `harness`). `ui.paint` (egui) is out of scope --
// it needs a live egui context, not a standalone pass.
//
//   cargo bench --bench pipeline            # full run
//   cargo bench --bench pipeline -- --test  # one pass each, no timing

mod harness;

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_pipeline(c: &mut Criterion) {
    let bench = harness::setup();
    bench.prime_frame();

    let mut group = c.benchmark_group("pipeline");

    let b = &bench;

    group.bench_function("clear", |t| {
        t.iter(|| b.time_pass(|cmd| b.clear.dispatch(cmd, b.frame.color())))
    });

    group.bench_function("line_depth_clear", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                cmd.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("LineDepthClear"),
                    color_attachments: &[Some(b.frame.line_depth().attachment_clear_far())],
                    ..Default::default()
                });
            })
        })
    });

    // --- Tractography acceleration structure (LineRenderer::transfer) ---

    group.bench_function("line/transform", |t| {
        t.iter(|| b.time_pass(|cmd| b.line_transform.dispatch(cmd, &b.asset, b.line())))
    });

    group.bench_function("line/crop", |t| {
        t.iter(|| b.time_pass(|cmd| b.line_crop.dispatch(cmd, &b.asset, b.line())))
    });

    group.bench_function("line/occupancy", |t| {
        t.iter(|| {
            b.time_pass(|cmd| b.line_occupancy.dispatch(cmd, &b.asset, &b.controller, b.line()))
        })
    });

    group.bench_function("line/cull", |t| {
        t.iter(|| b.time_pass(|cmd| b.line_cull.dispatch(cmd, &b.asset, &b.controller, b.line())))
    });

    group.bench_function("line/populate", |t| {
        t.iter(|| {
            b.time_pass(|cmd| b.line_populate.dispatch(cmd, &b.asset, &b.controller, b.line()))
        })
    });

    // --- Shared medium ---

    group.bench_function("volume/transfer_begin", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.volume_transfer.begin(
                    cmd,
                    &b.asset.volumes,
                    &b.asset.masks,
                    b.pv(),
                    &b.asset.crop,
                )
            })
        })
    });

    group.bench_function("volume/transfer_finalize", |t| {
        t.iter(|| b.time_pass(|cmd| b.volume_transfer.finalize(cmd, b.pv())))
    });

    group.bench_function("gradient", |t| {
        t.iter(|| b.time_pass(|cmd| b.gradient.dispatch(cmd, b.pv())))
    });

    group.bench_function("line/deposit", |t| {
        t.iter(|| b.time_pass(|cmd| b.line_deposit.dispatch(cmd, b.pv(), b.line())))
    });

    // --- Radiance cascade (see benches/cascade.rs for the per-level breakdown) ---

    group.bench_function("lighting/hdri", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.lighting.hdri(
                    cmd,
                    &b.asset.environment,
                    &b.asset.hdri,
                    b.radiance(),
                    b.pv(),
                )
            })
        })
    });

    group.bench_function("lighting/cascade_all", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.lighting.radiance(
                    cmd,
                    &b.asset.environment,
                    &b.asset.hdri,
                    b.radiance(),
                    b.pv(),
                )
            })
        })
    });

    // --- Raster traces ---

    group.bench_function("line/render", |t| {
        t.iter(|| b.time_pass(|cmd| b.line_render.dispatch(cmd, &b.asset, &b.controller, &b.frame)))
    });

    group.bench_function("volume/render", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.volume_render
                    .dispatch(&b.gpu, cmd, &b.asset, &b.controller, &b.frame)
            })
        })
    });

    // --- Resolve + present ---

    group.bench_function("accumulate", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.accumulate_buffer.set_sample(&b.gpu, 1);
                b.accumulate
                    .accumulate(cmd, &b.accumulate_buffer, &b.frame, 0)
            })
        })
    });

    group.bench_function("reduce", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.accumulate.reduce(
                    cmd,
                    &b.accumulate_buffer,
                    b.frame.accum(0),
                    b.frame.accum(1),
                )
            })
        })
    });

    group.bench_function("present", |t| {
        t.iter(|| b.time_pass(|cmd| b.present_frame(cmd)))
    });

    group.finish();
}

criterion_group!(benches, bench_pipeline);
criterion_main!(benches);
