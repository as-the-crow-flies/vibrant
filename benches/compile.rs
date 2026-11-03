// Cold-compiles every pipeline (all variants) once and prints how long each
// took, plus the total. Shaders are salted so the driver's shader cache can't
// serve them; every run adds fresh entries to that cache.
//
//   cargo bench --bench compile

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pollster::FutureExt;
use vibrant::{
    asset::Asset,
    gpu::{warmup, Gpu},
    renderer::{
        accumulate::AccumulatePipeline,
        gradient::GradientPipeline,
        lighting::LightingRenderer,
        line::{
            crop::LineCropPipeline, deposit::LineDepositPipeline, occupancy::LineOccupancyPipeline,
            populate::LinePopulatePipeline, render::LineRenderPipeline,
            transform::LineTransformPipeline,
        },
        present::PresentPipeline,
        util::clear::ClearPipeline,
        volume::{render::GaussianVolumeRenderer, transfer::VolumeTransferPipeline},
    },
};
use web_time::Instant;
use wgpu::TextureFormat;

fn main() {
    let gpu = Gpu::new().block_on().expect("Could not acquire a GPU");

    let salt = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    gpu.set_shader_salt(salt);

    let start = Instant::now();

    // Kept alive until every queued pipeline is built.
    let _pipelines = (
        Asset::new(&gpu),
        ClearPipeline::new(&gpu),
        AccumulatePipeline::new(&gpu),
        PresentPipeline::new(&gpu, TextureFormat::Bgra8Unorm, TextureFormat::Rgba16Float),
        VolumeTransferPipeline::new(&gpu),
        GradientPipeline::new(&gpu),
        GaussianVolumeRenderer::new(&gpu),
        LightingRenderer::new(&gpu),
        (
            LineTransformPipeline::new(&gpu),
            LineCropPipeline::new(&gpu),
            LineOccupancyPipeline::new(&gpu),
            LinePopulatePipeline::new(&gpu),
            LineDepositPipeline::new(&gpu),
            LineRenderPipeline::new(&gpu),
        ),
    );

    while warmup::pending() {
        warmup::step();
    }

    let wall = start.elapsed();
    let mut builds = warmup::take_builds();
    builds.sort_by(|(_, a), (_, b)| b.cmp(a));

    let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
    let width = builds
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);

    for (label, duration) in &builds {
        println!("{label:<width$}  {:>8.1} ms", ms(*duration));
    }

    let total: Duration = builds.iter().map(|(_, duration)| *duration).sum();
    println!("{:-<1$}", "", width + 13);
    println!(
        "{:<width$}  {:>8.1} ms",
        format!("total ({} pipelines)", builds.len()),
        ms(total)
    );
    println!("{:<width$}  {:>8.1} ms", "wall (incl. setup)", ms(wall));
}
