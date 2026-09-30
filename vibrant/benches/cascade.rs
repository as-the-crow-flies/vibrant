// Times the gaussian radiance-cascade compute pass (cascade.wgsl) in isolation:
// the HDRI fit, all levels together, and each level on its own. Scene setup is
// shared with `benches/pipeline.rs` (see `harness`). This bench leaves the
// LightingRenderer at its default 32-lobe variant.
//
//   cargo bench --bench cascade            # full run
//   cargo bench --bench cascade -- --test  # one pass + the histogram, no timing

mod harness;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pollster::FutureExt;
use vibrant::asset::radiance::GaussianRadianceBuffer;

use harness::Bench;

// Zeroes the EM-iteration histogram, dispatches one hdri+radiance pass, and
// prints how many probes converged at each iteration count per cascade level
// plus the hdri fit itself -- run outside criterion's timing loop since it's
// a one-shot diagnostic, not a benchmark. Bucket 0 is culled probes (they
// never enter the EM loop; hdri.wgsl has no cull step, so its row never has
// one). Useful for deciding whether EM_ITERATIONS_MAX/MIN/EM_CONVERGENCE in
// cascade.wgsl/hdri.wgsl can be tightened, and for seeing how much cull() is
// actually skipping.
fn print_em_iteration_histogram(bench: &Bench) {
    let buckets = GaussianRadianceBuffer::EM_HISTOGRAM_BUCKETS as usize;
    let rows = GaussianRadianceBuffer::EM_HISTOGRAM_ROWS as usize;
    let zeros = vec![0u8; rows * buckets * 4];
    bench
        .gpu
        .queue()
        .write_buffer(bench.radiance().em_iterations(), 0, &zeros);

    bench.time_pass(|cmd| {
        bench.lighting.hdri(
            cmd,
            &bench.asset.environment,
            &bench.asset.hdri,
            bench.radiance(),
            bench.pv(),
        );
        bench.lighting.radiance(
            cmd,
            &bench.asset.environment,
            &bench.asset.hdri,
            bench.radiance(),
            bench.pv(),
        );
    });

    let counts: Vec<u32> = bench
        .gpu
        .read_buffer(bench.radiance().em_iterations())
        .block_on()
        .expect("em_iterations buffer should still be alive during the benchmark");

    let print_row = |label: &str, row: &[u32]| {
        let total: u32 = row.iter().sum();
        if total == 0 {
            return;
        }

        let breakdown = row
            .iter()
            .map(|&count| 100 * count / total)
            .enumerate()
            .filter(|(_, count)| *count > 0)
            .map(|(iterations, count)| format!("{iterations}:{:?}%", count))
            .collect::<Vec<_>>()
            .join("  ");

        println!("  {label} ({total} probes): {breakdown}");
    };

    println!("\nEM iterations-to-converge:");

    let hdri_row = &counts[GaussianRadianceBuffer::LEVELS as usize * buckets..rows * buckets];
    print_row("hdri", hdri_row);

    for cascade in (0..GaussianRadianceBuffer::LEVELS as usize).rev() {
        let row = &counts[cascade * buckets..(cascade + 1) * buckets];
        print_row(&format!("cascade {cascade}"), row);
    }
    println!();
}

fn bench_cascade(c: &mut Criterion) {
    let bench = harness::setup();

    print_em_iteration_histogram(&bench);

    let b = &bench;

    // Just the HDRI fit: a single EM dispatch (workgroup of 1024, SAMPLES=4096)
    // that seeds CASCADE_MAX. Runs once per recompute, not once per probe --
    // kept as its own benchmark (outside the "cascade" group) since it's a
    // distinct pass with its own cost profile.
    c.bench_function("hdri", |t| {
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
        });
    });

    let mut group = c.benchmark_group("cascade");

    // The full radiance pass: the HDRI fit plus all 6 cascades, exactly what a
    // real recompute frame dispatches (see LightingRenderer::dispatch).
    group.bench_function("all_levels", |t| {
        t.iter(|| {
            b.time_pass(|cmd| {
                b.lighting.hdri(
                    cmd,
                    &b.asset.environment,
                    &b.asset.hdri,
                    b.radiance(),
                    b.pv(),
                );
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
    // WORKGROUP/SUBGROUPS table at the top of cascade.wgsl), so isolate each one.
    // CASCADE_MAX (level 5) seeds its per-probe EM from the "hdri" fit above
    // rather than starting cold, but still runs its own refinement per probe.
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
