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
    let hdri = HdriBuffer::new(&gpu);

    Scene {
        volume,
        radiance,
        renderer,
        environment,
        hdri,
        gpu,
    }
}

fn bench_cascade(c: &mut Criterion) {
    let scene = setup();

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
                    scene.renderer.dispatch_cascade(
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
