// Times the gaussian radiance-cascade compute pass (cascade.wgsl) in isolation.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{
        colormap::Colormap, crop::CropBuffer, environment::Environment, hdri::HdriBuffer,
        radiance::GaussianRadianceBuffer, volume::PhysicalVolume,
        volume_fraction::VolumeFractionBuffer, volume_mask::VolumeMaskBuffer,
    },
    file::{File, VolumeFile},
    gpu::Gpu,
    renderer::{
        gradient::GradientPipeline, lighting::LightingRenderer,
        volume::transfer::VolumeTransferPipeline,
    },
};

const RESOLUTION_DIVISOR: u32 = 4;

const VOLUME_PATH: &str = "/Users/bkraaijeveld/Data/HCP-100307/100307_t1w.nii.gz";

struct Scene {
    volume: PhysicalVolume,
    radiance: GaussianRadianceBuffer,
    lighting: LightingRenderer,
    environment: Environment,
    hdri: HdriBuffer,
    gpu: Gpu,
}

fn setup() -> Scene {
    let gpu = Gpu::new().block_on();

    let file = VolumeFile::from_comressed_nifti(&File::from(VOLUME_PATH));

    let colormap = Colormap::new(&gpu);
    // Named (not temporary) so they aren't dropped -- and their textures destroyed
    // via VolumeFractionBuffer/VolumeMaskBuffer's Drop impls -- before gpu.wait()
    // actually runs the transfer pass that reads them.
    let fractions = [VolumeFractionBuffer::new(&gpu, &file, &colormap)];
    let masks = [VolumeMaskBuffer::none(&gpu)];
    let crop = CropBuffer::new(&gpu);

    let volume = PhysicalVolume::new(&gpu, file.size(), file.transform());

    let transfer = VolumeTransferPipeline::new(&gpu);
    let gradient = GradientPipeline::new(&gpu);

    let mut cmd = gpu.cmd();
    transfer.dispatch(&mut cmd, &fractions, &masks, &volume, &crop);
    gradient.dispatch(&mut cmd, &volume);
    gpu.submit(cmd);
    gpu.wait();

    let radiance = GaussianRadianceBuffer::new(&gpu, file.size() / RESOLUTION_DIVISOR);
    let lighting = LightingRenderer::new(&gpu);
    let environment = Environment::new(&gpu);
    let mut hdri = HdriBuffer::new(&gpu);
    hdri.index = 3; // Ferndale

    let scene = Scene {
        volume,
        radiance,
        lighting,
        environment,
        hdri,
        gpu,
    };

    // CASCADE_MAX no longer fits itself against the HDRI -- it just broadcasts
    // whatever's in VMM_HDRI/PHI_HDRI (see cascade.wgsl). Prime those once here
    // so the per-cascade and histogram passes below aren't reading zeroed buffers.
    let mut cmd = scene.gpu.cmd();
    scene.lighting.hdri(
        &mut cmd,
        &scene.environment,
        &scene.hdri,
        &scene.radiance,
        &scene.volume,
    );
    scene.gpu.submit(cmd);
    scene.gpu.wait();

    scene
}

// Zeroes the EM-iteration histogram, dispatches one hdri+radiance pass, and
// prints how many probes converged at each iteration count per cascade level
// plus the hdri fit itself -- run outside criterion's timing loop since it's
// a one-shot diagnostic, not a benchmark. Bucket 0 is culled probes (they
// never enter the EM loop; hdri.wgsl has no cull step, so its row never has
// one). Useful for deciding whether EM_ITERATIONS_MAX/MIN/EM_CONVERGENCE in
// cascade.wgsl/hdri.wgsl can be tightened, and for seeing how much cull() is
// actually skipping.
fn print_em_iteration_histogram(scene: &Scene) {
    let buckets = GaussianRadianceBuffer::EM_HISTOGRAM_BUCKETS as usize;
    let rows = GaussianRadianceBuffer::EM_HISTOGRAM_ROWS as usize;
    let zeros = vec![0u8; rows * buckets * 4];
    scene
        .gpu
        .queue()
        .write_buffer(scene.radiance.em_iterations(), 0, &zeros);

    let mut cmd = scene.gpu.cmd();
    scene.lighting.hdri(
        &mut cmd,
        &scene.environment,
        &scene.hdri,
        &scene.radiance,
        &scene.volume,
    );
    scene.lighting.radiance(
        &mut cmd,
        &scene.environment,
        &scene.hdri,
        &scene.radiance,
        &scene.volume,
    );
    scene.gpu.submit(cmd);
    scene.gpu.wait();

    let counts: Vec<u32> = scene
        .gpu
        .read_buffer(scene.radiance.em_iterations())
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
    let scene = setup();

    print_em_iteration_histogram(&scene);

    // Just the HDRI fit: a single EM dispatch (workgroup of 1024, SAMPLES=4096)
    // that seeds CASCADE_MAX. Runs once per recompute, not once per probe --
    // kept as its own benchmark (outside the "cascade" group) since it's a
    // distinct pass with its own cost profile.
    c.bench_function("hdri", |b| {
        b.iter(|| {
            let mut cmd = scene.gpu.cmd();
            scene.lighting.hdri(
                &mut cmd,
                &scene.environment,
                &scene.hdri,
                &scene.radiance,
                &scene.volume,
            );
            scene.gpu.submit(cmd);
            scene.gpu.wait();
        });
    });

    let mut group = c.benchmark_group("cascade");

    // The full radiance pass: the HDRI fit plus all 6 cascades, exactly what a
    // real recompute frame dispatches (see LightingRenderer::dispatch).
    group.bench_function("all_levels", |b| {
        b.iter(|| {
            let mut cmd = scene.gpu.cmd();
            scene.lighting.hdri(
                &mut cmd,
                &scene.environment,
                &scene.hdri,
                &scene.radiance,
                &scene.volume,
            );
            scene.lighting.radiance(
                &mut cmd,
                &scene.environment,
                &scene.hdri,
                &scene.radiance,
                &scene.volume,
            );
            scene.gpu.submit(cmd);
            scene.gpu.wait();
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
            |b, &cascade| {
                b.iter(|| {
                    let mut cmd = scene.gpu.cmd();
                    scene.lighting.cascade(
                        &mut cmd,
                        &scene.environment,
                        &scene.hdri,
                        &scene.radiance,
                        &scene.volume,
                        cascade,
                    );
                    scene.gpu.submit(cmd);
                    scene.gpu.wait();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_cascade);
criterion_main!(benches);
