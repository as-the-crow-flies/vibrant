// Times every GPU pass `Renderer::render` dispatches per frame, in render order,
// against one real scene (see `harness`). `ui.paint` (egui) is out of scope --
// it needs a live egui context, not a standalone pass.
//
//   cargo bench --bench pipeline            # full run
//   cargo bench --bench pipeline -- --test  # one pass each, no timing
//   ALPHA=0.1 cargo bench --bench pipeline -- line/trace  # line opacity (default 1)
//   TRACED=1 cargo bench --bench pipeline -- volume/trace  # ray traced lighting
//   TRACED=1 MIRROR=1 cargo bench --bench pipeline -- volume/trace  # with mirror Fresnel

mod harness;

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_pipeline(c: &mut Criterion) {
    let mut bench = harness::setup();
    bench.controller.settings_mut().ray_traced = std::env::var("TRACED").is_ok();
    bench.controller.settings_mut().mirror_fresnel = std::env::var("MIRROR").is_ok();
    bench.prime_frame();

    let mut group = c.benchmark_group("pipeline");

    let b = &bench;
    for pass in b.passes() {
        group.bench_function(pass.name, |t| {
            t.iter(|| b.time_pass(|cmd| (pass.record)(cmd)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_pipeline);
criterion_main!(benches);
