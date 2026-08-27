// Times the gaussian radiance-cascade compute pass (cascade.wgsl) in isolation.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::{
        colormap::Colormap, crop::CropBuffer, hdri::HdriBuffer,
        radiance::gaussian::GaussianRadianceBuffer, volume::PhysicalVolume,
        volume_fraction::VolumeFractionBuffer, volume_mask::VolumeMaskBuffer,
    },
    file::{File, VolumeFile},
    gpu::Gpu,
    renderer::{
        anatomy::{
            gradient::GradientPipeline, render::gaussian::GaussianVolumeRenderer,
            transfer::AnatomyTransferPipeline,
        },
        environment::Environment,
    },
};

const RESOLUTION_DIVISOR: u32 = 4;

const VOLUME_PATH: &str = "/Users/bkraaijeveld/Data/HCP-100307/100307_t1w.nii.gz";

struct Scene {
    volume: PhysicalVolume,
    radiance: GaussianRadianceBuffer,
    renderer: GaussianVolumeRenderer,
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

    let transfer = AnatomyTransferPipeline::new(&gpu);
    let gradient = GradientPipeline::new(&gpu);

    let mut cmd = gpu.cmd();
    transfer.dispatch(&mut cmd, &fractions, &masks, &volume, &crop);
    gradient.dispatch(&mut cmd, &volume);
    gpu.submit(cmd);
    gpu.wait();

    let radiance = GaussianRadianceBuffer::new(&gpu, file.size() / RESOLUTION_DIVISOR);
    let renderer = GaussianVolumeRenderer::new(&gpu);
    let environment = Environment::new(&gpu);
    let mut hdri = HdriBuffer::new(&gpu);
    hdri.index = 3; // Ferndale

    Scene {
        volume,
        radiance,
        renderer,
        environment,
        hdri,
        gpu,
    }
}

// Zeroes the EM-iteration histogram, dispatches one full radiance pass, and
// prints how many probes converged at each iteration count per cascade level --
// run outside criterion's timing loop since it's a one-shot diagnostic, not a
// benchmark. Bucket 0 is culled probes (they never enter the EM loop). Useful
// for deciding whether EM_ITERATIONS_MAX/MIN/EM_CONVERGENCE in cascade.wgsl can
// be tightened, and for seeing how much cull() is actually skipping.
fn print_em_iteration_histogram(scene: &Scene) {
    let buckets = GaussianRadianceBuffer::EM_HISTOGRAM_BUCKETS as usize;
    let zeros = vec![0u8; GaussianRadianceBuffer::LEVELS as usize * buckets * 4];
    scene
        .gpu
        .queue()
        .write_buffer(scene.radiance.em_iterations(), 0, &zeros);

    let mut cmd = scene.gpu.cmd();
    scene.renderer.radiance(
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
        .block_on();

    println!("\nEM iterations-to-converge, by cascade level:");
    for cascade in (0..GaussianRadianceBuffer::LEVELS as usize).rev() {
        let row = &counts[cascade * buckets..(cascade + 1) * buckets];
        let total: u32 = row.iter().sum();
        if total == 0 {
            continue;
        }

        let breakdown = row
            .iter()
            .map(|&count| 100 * count / total)
            .enumerate()
            .filter(|(_, count)| *count > 0)
            .map(|(iterations, count)| format!("{iterations}:{:?}%", count))
            .collect::<Vec<_>>()
            .join("  ");

        println!("  cascade {cascade} ({total} probes): {breakdown}");
    }
    println!();
}

fn bench_cascade(c: &mut Criterion) {
    let scene = setup();

    print_em_iteration_histogram(&scene);

    let mut group = c.benchmark_group("cascade");

    // The full radiance pass: all 6 cascades in one compute pass, exactly what a
    // real recompute frame dispatches.
    group.bench_function("all_levels", |b| {
        b.iter(|| {
            let mut cmd = scene.gpu.cmd();
            scene.renderer.radiance(
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
    for cascade in 0..GaussianRadianceBuffer::LEVELS as usize {
        group.bench_with_input(
            BenchmarkId::new("level", cascade),
            &cascade,
            |b, &cascade| {
                b.iter(|| {
                    let mut cmd = scene.gpu.cmd();
                    scene.renderer.cascade(
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
