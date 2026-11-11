// Times the gaussian radiance-cascade compute pass (cascade.wgsl) in isolation:
// all levels together, and each level on its own. Scene setup is
// shared with `benches/pipeline.rs` (see `harness`).
//
//   cargo bench --bench cascade            # full run
//   cargo bench --bench cascade -- --test  # one pass, no timing

mod harness;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use vibrant::asset::radiance::GaussianRadianceBuffer;

fn bench_cascade(c: &mut Criterion) {
    let bench = harness::setup();

    let b = &bench;

    let mut group = c.benchmark_group("cascade");

    // The full radiance pass: all 6 cascades, exactly what a real recompute
    // frame dispatches (see LightingRenderer::dispatch_for).
    group.bench_function("all_levels", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.lighting.radiance(
                    cmd,
                    &b.asset.environment,
                    &b.asset.hdri,
                    b.radiance(),
                    b.pv(),
                );
            })
        });
    });

    // Per-level breakdown: cascades differ hugely in cost (see the LEVEL/SAMPLES/
    // WORKGROUP table at the top of cascade.wgsl), so isolate each one.
    // CASCADE_MAX (level 5) seeds its per-probe EM from the HDRI fit baked at
    // load rather than starting cold, but still runs its own refinement per probe.
    for cascade in 0..GaussianRadianceBuffer::LEVELS as usize {
        group.bench_with_input(
            BenchmarkId::new("level", cascade),
            &cascade,
            |t, &cascade| {
                t.iter(|| {
                    b.time_pass(|cmd| {
                        b.lighting.cascade(
                            cmd,
                            &b.asset.environment,
                            &b.asset.hdri,
                            b.radiance(),
                            b.pv(),
                            cascade,
                        )
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_cascade);
criterion_main!(benches);
