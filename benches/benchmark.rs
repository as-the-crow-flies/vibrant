use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;
use vibrant::{
    asset::line::LineSet,
    controller::{
        settings::{LineVoxelizationMode, Settings},
        Controller,
    },
    file::LineFile,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::{
            culling::LineCullingPipeline, occlusion::LineOcclusionPipeline,
            occupancy::LineOccupancyPipeline, populate::LinePopulatePipeline,
            render::hybrid::HybridLineRenderPipeline, transform::LineTransformPipeline,
        },
    },
    surface::Frame,
};

pub fn empty(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    criterion.bench_function("empty", |bencher| {
        bencher.iter(|| {
            let cmd = gpu.cmd();
            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn density(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &get_tractogram(gpu);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).render(&mut cmd, environment, tractogram);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = LineOccupancyPipeline::new(gpu);

    criterion.bench_function("density", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(
                &mut cmd,
                frame,
                environment,
                LineVoxelizationMode::Tube,
                tractogram,
            );

            gpu.submit(cmd);
            gpu.wait();
        })
    });

    let density: Vec<u32> = gpu
        .read_buffer(frame.occupancy().occupancy_count_buffer())
        .block_on();
    let u14_max = 2u32.pow(14) - 1;

    let counts: Vec<u32> = density
        .iter()
        .map(|value| value & u14_max)
        .filter(|&value| value > 0)
        .collect();

    let max_count = counts.iter().max();
    let mean_count = counts.iter().sum::<u32>() as usize / counts.len();

    dbg!(max_count, mean_count);
}

pub fn occlusion(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &get_tractogram(gpu);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).render(&mut cmd, environment, tractogram);
    LineOccupancyPipeline::new(gpu).render(
        &mut cmd,
        frame,
        environment,
        LineVoxelizationMode::Tube,
        tractogram,
    );
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = LineOcclusionPipeline::new(gpu);

    criterion.bench_function("occlusion", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn occupancy(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &get_tractogram(gpu);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).render(&mut cmd, environment, tractogram);
    LineOccupancyPipeline::new(gpu).render(
        &mut cmd,
        frame,
        environment,
        LineVoxelizationMode::Tube,
        tractogram,
    );
    LineOcclusionPipeline::new(gpu).render(&mut cmd, frame, environment);
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = LineCullingPipeline::new(gpu);

    criterion.bench_function("occupancy", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn populate(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &get_tractogram(gpu);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).render(&mut cmd, environment, tractogram);
    LineOccupancyPipeline::new(gpu).render(
        &mut cmd,
        frame,
        environment,
        LineVoxelizationMode::Tube,
        tractogram,
    );
    LineOcclusionPipeline::new(gpu).render(&mut cmd, frame, environment);
    gpu.submit(cmd);
    gpu.wait();

    let occupancy = LineCullingPipeline::new(gpu);
    let populate = LinePopulatePipeline::new(gpu);

    criterion.bench_function("populate", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            occupancy.render(&mut cmd, frame, environment);
            populate.render(
                &mut cmd,
                frame,
                environment,
                LineVoxelizationMode::Tube,
                tractogram,
            );

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn render(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &get_tractogram(gpu);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).render(&mut cmd, environment, tractogram);
    LineOccupancyPipeline::new(gpu).render(
        &mut cmd,
        frame,
        environment,
        LineVoxelizationMode::Tube,
        tractogram,
    );
    LineOcclusionPipeline::new(gpu).render(&mut cmd, frame, environment);
    LineCullingPipeline::new(gpu).render(&mut cmd, frame, environment);
    LinePopulatePipeline::new(gpu).render(
        &mut cmd,
        frame,
        environment,
        LineVoxelizationMode::Tube,
        tractogram,
    );
    gpu.submit(cmd);
    gpu.wait();

    let pipeline = HybridLineRenderPipeline::new(gpu);

    criterion.bench_function("render", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            pipeline.render(&mut cmd, frame, environment, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

pub fn full(criterion: &mut Criterion) {
    let gpu = &Gpu::new().block_on();

    let environment = &get_environment(gpu);
    let frame = &Frame::new(gpu, &Settings::new());
    let tractogram = &get_tractogram(gpu);

    let mut cmd = gpu.cmd();
    LineTransformPipeline::new(gpu).render(&mut cmd, environment, tractogram);
    gpu.submit(cmd);
    gpu.wait();

    let density = LineOccupancyPipeline::new(gpu);
    let occlusion = LineOcclusionPipeline::new(gpu);
    let occupancy = LineCullingPipeline::new(gpu);
    let populate = LinePopulatePipeline::new(gpu);
    let render = HybridLineRenderPipeline::new(gpu);

    criterion.bench_function("full", |bencher| {
        bencher.iter(|| {
            let mut cmd = gpu.cmd();

            density.render(
                &mut cmd,
                frame,
                environment,
                LineVoxelizationMode::Tube,
                tractogram,
            );
            occlusion.render(&mut cmd, frame, environment);
            occupancy.render(&mut cmd, frame, environment);
            populate.render(
                &mut cmd,
                frame,
                environment,
                LineVoxelizationMode::Tube,
                tractogram,
            );
            render.render(&mut cmd, frame, environment, tractogram);

            gpu.submit(cmd);
            gpu.wait();
        })
    });
}

criterion_group!(benches, empty, density, occlusion, occupancy, populate, render, full);
criterion_main!(benches);

pub fn get_tractogram(gpu: &Gpu) -> LineSet {
    LineSet::new(
        gpu,
        &LineFile::from_file("assets/3D_line_sets/ANEURYSM.obj"),
        // &LineFile::from_file("assets/HCP-100307/whole_brain1M.tck"),
        // &LineFile::from_file("assets/HCP-100307/whole_brain200k.tck"),
        // &LineFile::join(vec![
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_right.tck"),
        // ]),
        // &LineFile::join(vec![
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/AF_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ATR_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CA.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_1.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_2.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_3.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_4.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_5.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_6.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC_7.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CC.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CG_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/CST_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/FPT_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/FPT_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/FX_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/FX_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ICP_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ICP_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/IFO_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/IFO_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ILF_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ILF_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/MCP.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/MLF_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/MLF_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/OR_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/OR_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/POPT_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/POPT_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SCP_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SCP_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_I_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_I_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_II_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_II_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_III_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/SLF_III_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_FO_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_FO_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_OCC_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_OCC_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PAR_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PAR_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_POSTC_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_POSTC_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREC_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREC_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREF_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREF_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREM_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/ST_PREM_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/STR_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/STR_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_OCC_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_OCC_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PAR_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PAR_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_POSTC_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_POSTC_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREC_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREC_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREF_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREF_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREM_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/T_PREM_right.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/UF_left.tck"),
        //     LineFile::from_file("assets/HCP-100307/TOM_trackings/UF_right.tck"),
        // ]),
    )
}

pub fn get_environment(gpu: &Gpu) -> Environment {
    let environment = Environment::new(&gpu);
    environment.update(&gpu, &Controller::new());
    return environment;
}
