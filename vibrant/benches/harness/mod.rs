// Shared setup for the GPU benchmarks. Builds one real scene (volume + lines)
// the same way `Renderer` does, exposes every per-frame pass's pipeline, and a
// `time_pass` primitive that submits one command buffer and blocks on the GPU.
//
// Used by `benches/cascade.rs` (radiance cascade only) and `benches/pipeline.rs`
// (every stage). Kept out of the `[[bench]]` list -- it's a module, not a target.

#![allow(dead_code)] // Each bench binary uses a different subset.

use std::path::PathBuf;

use pollster::FutureExt;
use vibrant::{
    asset::{line::LineBuffer, radiance::GaussianRadianceBuffer, volume::PhysicalVolume, Asset},
    controller::Controller,
    file::FileStage,
    gpu::Gpu,
    renderer::{
        accumulate::AccumulatePipeline,
        gradient::GradientPipeline,
        lighting::LightingRenderer,
        line::{
            crop::LineCropPipeline, cull::LineCullPipeline, deposit::LineDepositPipeline,
            occupancy::LineOccupancyPipeline, populate::LinePopulatePipeline,
            render::LineRenderPipeline, transform::LineTransformPipeline,
        },
        present::{PresentPipeline, Presentation},
        util::clear::ClearPipeline,
        volume::{render::GaussianVolumeRenderer, transfer::VolumeTransferPipeline},
    },
    surface::{accumulate::AccumulateBuffer, Frame},
    Vec2,
};
use wgpu::{
    CommandEncoder, Extent3d, RenderPassDescriptor, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

/// Scene + render settings shared by every bench.
pub const SETTINGS: Settings = Settings {
    volume_path: "/Users/bkraaijeveld/Data/HCP-100307/100307_t1w.nii.gz",
    line_path: "/Users/bkraaijeveld/Data/HCP-100307/whole_brain200k.tck",
    lightmap_resolution: 128,
    lobes: 32,
    width: 1920,
    height: 1080,
    hdri_index: 3, // Ferndale
    hdri_rotation: 0.0,
    hdri_strength: 1.0,
};

pub struct Settings {
    pub volume_path: &'static str,
    pub line_path: &'static str,
    /// Radiance lightmap resolution (probes along the volume's longest axis).
    pub lightmap_resolution: u32,
    /// VMM lobe count -- one of `VMM_SIZE_OPTIONS`.
    pub lobes: u32,
    /// Render viewport the raster passes (volume trace, line trace, present) cover.
    pub width: u32,
    pub height: u32,
    pub hdri_index: usize,
    pub hdri_rotation: f32,
    pub hdri_strength: f32,
}

pub struct Bench {
    pub gpu: Gpu,
    pub controller: Controller,
    pub asset: Asset,
    pub frame: Frame,

    pub clear: ClearPipeline,
    pub line_transform: LineTransformPipeline,
    pub line_crop: LineCropPipeline,
    pub line_occupancy: LineOccupancyPipeline,
    pub line_cull: LineCullPipeline,
    pub line_populate: LinePopulatePipeline,
    pub line_deposit: LineDepositPipeline,
    pub line_render: LineRenderPipeline,
    pub volume_transfer: VolumeTransferPipeline,
    pub gradient: GradientPipeline,
    pub volume_render: GaussianVolumeRenderer,
    pub lighting: LightingRenderer,
    pub accumulate: AccumulatePipeline,
    pub accumulate_buffer: AccumulateBuffer,
    pub present: PresentPipeline,

    present_target: TextureView,
}

pub fn setup() -> Bench {
    let settings = &SETTINGS;

    let gpu = Gpu::new().block_on().expect("Could not acquire a GPU");

    // Push the scene files onto the global FileStage queue, then let `Asset`
    // drain + build them exactly as the app does on load.
    FileStage::load_path_blocking(&PathBuf::from(settings.volume_path));
    FileStage::load_path_blocking(&PathBuf::from(settings.line_path));

    let mut controller = Controller::new();
    controller.settings_mut().width = settings.width;
    controller.settings_mut().height = settings.height;
    controller.set_viewport(settings.width as f32, settings.height as f32);
    controller
        .rendering_mut()
        .set_lightmap_resolution(settings.lightmap_resolution);
    controller.rendering_mut().set_lobes(settings.lobes);

    let mut asset = Asset::new(&gpu);
    asset.update(&gpu, &mut controller); // builds line/volume/physical_volume/radiance
    asset.hdri.index = settings.hdri_index;
    asset.hdri.settings_mut().rotation = settings.hdri_rotation;
    asset.hdri.settings_mut().strength = settings.hdri_strength;
    asset.hdri.update_settings(&gpu);
    asset.environment.update(&gpu, &controller, Vec2::ZERO);

    let frame = Frame::new(&gpu, controller.settings());

    let present_target = gpu
        .device()
        .create_texture(&TextureDescriptor {
            label: Some("bench.present_target"),
            size: Extent3d {
                width: settings.width,
                height: settings.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default());

    let lighting = LightingRenderer::new(&gpu);
    lighting.set_vmm_size(settings.lobes);

    let bench = Bench {
        clear: ClearPipeline::new(&gpu),
        line_transform: LineTransformPipeline::new(&gpu),
        line_crop: LineCropPipeline::new(&gpu),
        line_occupancy: LineOccupancyPipeline::new(&gpu),
        line_cull: LineCullPipeline::new(&gpu),
        line_populate: LinePopulatePipeline::new(&gpu),
        line_deposit: LineDepositPipeline::new(&gpu),
        line_render: LineRenderPipeline::new(&gpu),
        volume_transfer: VolumeTransferPipeline::new(&gpu),
        gradient: GradientPipeline::new(&gpu),
        volume_render: GaussianVolumeRenderer::new(&gpu),
        lighting,
        accumulate: AccumulatePipeline::new(&gpu),
        accumulate_buffer: AccumulateBuffer::new(&gpu),
        present: PresentPipeline::new(&gpu, TextureFormat::Bgra8Unorm, TextureFormat::Rgba16Float),

        gpu,
        controller,
        asset,
        frame,
        present_target,
    };

    // Populate every 3D texture the passes read from -- the marched volume +
    // gradients, the tractography occupancy pyramid, the deposited line density,
    // and one full radiance-cascade solve -- so benchmarked passes measure
    // steady-state cost, not work against zeroed buffers (an unpopulated
    // extinction volume makes `is_culled()` reject every probe). `pipeline.rs`
    // additionally calls `prime_frame` to exercise the raster + resolve passes.
    bench.time_pass(|cmd| bench.prime_static(cmd));

    bench
}

impl Bench {
    /// Record one command buffer, submit it, block until the GPU is idle. The
    /// unit every `b.iter` body wraps.
    pub fn time_pass<F: FnMut(&mut CommandEncoder)>(&self, mut record: F) {
        let mut cmd = self.gpu.cmd();
        record(&mut cmd);
        self.gpu.submit(cmd);
        self.gpu.wait();
    }

    pub fn pv(&self) -> &PhysicalVolume {
        self.asset.physical_volume.as_ref().expect("volume loaded")
    }

    pub fn line(&self) -> &LineBuffer {
        self.asset.line.as_ref().expect("lines loaded")
    }

    pub fn radiance(&self) -> &GaussianRadianceBuffer {
        self.asset.radiance.as_ref().expect("radiance built")
    }

    pub fn present_target(&self) -> &TextureView {
        &self.present_target
    }

    /// Line accel structure + shared medium (marched volume, gradients,
    /// deposited line density). Shared by both prime paths.
    fn record_scene_build(&self, cmd: &mut CommandEncoder) {
        let (asset, pv, line) = (&self.asset, self.pv(), self.line());

        self.line_transform.dispatch(cmd, asset, line);
        self.line_crop.dispatch(cmd, asset, line);
        self.line_occupancy
            .dispatch(cmd, asset, &self.controller, line);
        self.line_cull.dispatch(cmd, asset, &self.controller, line);
        self.line_populate
            .dispatch(cmd, asset, &self.controller, line);

        self.volume_transfer
            .begin(cmd, &asset.volumes, &asset.masks, pv, &asset.crop);
        self.volume_transfer.finalize(cmd, pv);
        self.gradient.dispatch(cmd, pv);
        self.line_deposit.dispatch(cmd, pv, line);
    }

    /// One-time population run from `setup`: scene build + a full radiance solve
    /// via the direct calls `cascade.rs` benchmarks.
    fn prime_static(&self, cmd: &mut CommandEncoder) {
        self.record_scene_build(cmd);

        let (asset, pv) = (&self.asset, self.pv());
        self.lighting
            .hdri(cmd, &asset.environment, &asset.hdri, self.radiance(), pv);
        self.lighting
            .radiance(cmd, &asset.environment, &asset.hdri, self.radiance(), pv);
    }

    /// Replay the whole `Renderer::render` pass block once. `pipeline.rs` runs
    /// this a couple of times before timing so every texture / cascade / accel
    /// buffer holds steady-state data.
    pub fn prime_frame(&self) {
        for _ in 0..2 {
            self.time_pass(|cmd| self.record_frame(cmd));
        }
    }

    fn record_frame(&self, cmd: &mut CommandEncoder) {
        let (asset, frame) = (&self.asset, &self.frame);
        let pv = self.pv();

        self.clear.dispatch(cmd, frame.color());
        cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some("LineDepthClear"),
            color_attachments: &[Some(frame.line_depth().attachment_clear_far())],
            ..Default::default()
        });

        self.record_scene_build(cmd);

        self.lighting
            .dispatch_for(cmd, asset, &self.controller, self.radiance(), pv);

        // Combined mode: opaque lines first, then the volume clamped to them.
        self.line_render
            .dispatch(cmd, asset, &self.controller, frame);
        self.volume_render
            .dispatch(&self.gpu, cmd, asset, &self.controller, frame);

        // Fold into the running mean.
        self.accumulate_buffer.set_sample(&self.gpu, 1);
        self.accumulate
            .accumulate(cmd, &self.accumulate_buffer, frame, 0);
    }

    /// The on-screen present: tone map `frame.color()` + overlay into an
    /// off-screen `Bgra8Unorm` target (stands in for the swapchain image).
    pub fn present_frame(&self, cmd: &mut CommandEncoder) {
        self.present.set_headroom(&self.gpu, 1.0);
        self.present.dispatch(
            cmd,
            self.frame.color(),
            &self.frame,
            &self.present_target,
            Presentation::Sdr,
        );
    }
}
