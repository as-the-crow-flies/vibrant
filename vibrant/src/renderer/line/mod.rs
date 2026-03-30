pub mod aa;
pub mod crop;
pub mod cull;
pub mod foveated;
pub mod occlusion;
pub mod occupancy;
pub mod populate;
pub mod post;
pub mod render;
pub mod transform;

use occupancy::LineOccupancyPipeline;
use wgpu::{CommandEncoder, TextureFormat};

use aa::AntiAliasingPipeline;
use foveated::FoveatedCompositePipeline;

use crate::{
    asset::{line::LineBuffer, transform::TransformBuffer},
    controller::settings::{LineVoxelizationMode, Settings},
    gpu::Gpu,
    renderer::{
        line::{
            crop::LineCropPipeline, cull::LineCullPipeline, occlusion::LineOcclusionPipeline,
            populate::LinePopulatePipeline, post::PostProcessingPipeline,
            render::LineRenderPipeline, transform::LineTransformPipeline,
        },
        profiler::{
            GpuProfiler, PASS_AA, PASS_CROP, PASS_CULL, PASS_OCCLUSION, PASS_OCCUPANCY,
            PASS_POPULATE, PASS_POST, PASS_RENDER, PASS_TRANSFORM,
        },
    },
    surface::color::ColorBuffer,
    surface::Frame,
};

use super::environment::Environment;

/// Captures every setting that influences the voxel-segment spatial index
/// (OFFSET + INDEX buffers).  When this key is identical to the previous frame,
/// populate can be skipped and the cached index reused.
///
/// Camera-dependent fields (view/projection matrices) are intentionally absent:
/// the index is a pure function of scene geometry and voxelization parameters.
#[derive(PartialEq, Clone)]
struct VoxelizationKey {
    // f32 fields stored as bits for exact equality (NaN-safe, no fp comparison)
    radius: u32,
    smoothing: u32,
    volume: u32,
    voxelization: LineVoxelizationMode,
    crop_start: u32,
    crop_end: u32,
    crop_x_start: u32,
    crop_x_end: u32,
    crop_y_start: u32,
    crop_y_end: u32,
    crop_z_start: u32,
    crop_z_end: u32,
    /// Bumped whenever per-line visibility or crop settings change.
    line_settings_gen: u64,
}

impl VoxelizationKey {
    fn from(settings: &Settings, line: &LineBuffer) -> Self {
        Self {
            radius: settings.radius.to_bits(),
            smoothing: settings.smoothing.to_bits(),
            volume: settings.volume,
            voxelization: settings.voxelization,
            crop_start: settings.crop_start.to_bits(),
            crop_end: settings.crop_end.to_bits(),
            crop_x_start: settings.crop_x_start.to_bits(),
            crop_x_end: settings.crop_x_end.to_bits(),
            crop_y_start: settings.crop_y_start.to_bits(),
            crop_y_end: settings.crop_y_end.to_bits(),
            crop_z_start: settings.crop_z_start.to_bits(),
            crop_z_end: settings.crop_z_end.to_bits(),
            line_settings_gen: line.settings_generation,
        }
    }
}

pub struct LineRenderer {
    transform: LineTransformPipeline,
    crop: LineCropPipeline,
    occupancy: LineOccupancyPipeline,
    cull: LineCullPipeline,
    occlusion: LineOcclusionPipeline,
    populate: LinePopulatePipeline,
    render: LineRenderPipeline,
    post: PostProcessingPipeline,
    // Anti-aliasing pass: runs after post-processing, writes to frame.aa.
    aa: AntiAliasingPipeline,
    // Foveated rendering composite pass
    foveated: FoveatedCompositePipeline,
    // Per-pass GPU timestamp profiler.
    profiler: GpuProfiler,
    // last voxelization key for which the spatial index was built
    // None => index has never been built (forces populate on the first frame)
    prev_voxelization_key: Option<VoxelizationKey>,
    // ID of the Frame whose CullingBuffer was last populated
    // when the Frame is recreated (resize, foveated toggle) its ID changes,
    // invalidating the cached OFFSET/INDEX buffers
    last_frame_id: u64,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self::new_with_format(gpu, ColorBuffer::FORMAT)
    }

    pub fn new_with_format(gpu: &Gpu, format: TextureFormat) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),
            crop: LineCropPipeline::new(gpu),
            occupancy: LineOccupancyPipeline::new(gpu),
            occlusion: LineOcclusionPipeline::new(gpu),
            cull: LineCullPipeline::new(gpu),
            populate: LinePopulatePipeline::new(gpu),
            render: LineRenderPipeline::new_with_format(gpu, format),
            post: PostProcessingPipeline::new_with_format(gpu, format),
            aa: AntiAliasingPipeline::new_with_format(gpu, format),
            foveated: FoveatedCompositePipeline::new_with_format(gpu, format),
            profiler: GpuProfiler::new(gpu),
            prev_voxelization_key: None,
            last_frame_id: 0,
        }
    }

    pub fn reset_taa_history(&mut self) {
        self.aa.reset_taa_history();
    }

    pub fn render(
        &mut self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        line: &LineBuffer,
        transform: &TransformBuffer,
        settings: &Settings,
        needs_transform: bool,
        _needs_update: bool,
    ) {
        self.profiler.begin(cmd, PASS_TRANSFORM);
        if needs_transform {
            self.transform.dispatch(cmd, line, transform, environment);
        }
        self.profiler.end(cmd, PASS_TRANSFORM);

        self.profiler.begin(cmd, PASS_CROP);
        self.crop.dispatch(cmd, line, environment);
        self.profiler.end(cmd, PASS_CROP);

        self.profiler.begin(cmd, PASS_OCCUPANCY);
        self.occupancy
            .dispatch(cmd, frame, environment, settings, line);
        self.profiler.end(cmd, PASS_OCCUPANCY);

        self.profiler.begin(cmd, PASS_CULL);
        self.cull.dispatch(cmd, frame, environment);
        self.profiler.end(cmd, PASS_CULL);

        self.profiler.begin(cmd, PASS_OCCLUSION);
        self.occlusion.dispatch(cmd, frame, environment);
        self.profiler.end(cmd, PASS_OCCLUSION);

        let vox_key = VoxelizationKey::from(settings, line);
        let needs_populate = !settings.populate_cache
            || needs_transform
            || self.prev_voxelization_key.as_ref() != Some(&vox_key)
            || frame.id() != self.last_frame_id;

        self.profiler.begin(cmd, PASS_POPULATE);
        if needs_populate {
            self.prev_voxelization_key = Some(vox_key);
            self.last_frame_id = frame.id();
            self.populate
                .dispatch(cmd, frame, environment, settings, line);
        }
        self.profiler.end(cmd, PASS_POPULATE);

        self.profiler.begin(cmd, PASS_RENDER);
        if settings.foveated {
            // Pass 1: peripheral (low-res) → frame.foveated_peripheral
            if let Some(peripheral_buf) = frame.foveated_peripheral() {
                self.render.dispatch_to_target(
                    cmd,
                    environment.peripheral_binding(),
                    frame,
                    peripheral_buf,
                    line,
                    settings,
                );
            }

            // Pass 2: focus (high-res, sub-frustum) → frame.foveated_focus
            if let Some(focus_buf) = frame.foveated_focus() {
                self.render.dispatch_to_target(
                    cmd,
                    environment.focus_binding(),
                    frame,
                    focus_buf,
                    line,
                    settings,
                );
            }

            // Pass 3: composite peripheral + focus → frame.color (normal resolution)
            self.foveated.dispatch(cmd, environment, frame);
        } else {
            self.render
                .dispatch(cmd, environment, frame, line, settings);
        }
        self.profiler.end(cmd, PASS_RENDER);

        self.profiler.begin(cmd, PASS_POST);
        self.post.dispatch(cmd, environment, frame, settings);
        self.profiler.end(cmd, PASS_POST);

        self.profiler.begin(cmd, PASS_AA);
        self.aa.dispatch(cmd, environment, frame, settings);
        self.profiler.end(cmd, PASS_AA);

        self.profiler.resolve(cmd);
    }

    pub fn collect_profile(&mut self, gpu: &Gpu) {
        self.profiler.collect(gpu);
    }

    pub fn profile_results(&self) -> Vec<(&str, f32)> {
        self.profiler
            .labels()
            .iter()
            .zip(self.profiler.results.iter())
            .map(|(label, ms)| (*label, *ms))
            .collect()
    }

    pub fn profiler_enabled(&self) -> bool {
        self.profiler.enabled()
    }
}
