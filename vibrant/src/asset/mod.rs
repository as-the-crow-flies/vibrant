pub mod colormap;
pub mod crop;
pub mod environment;
pub mod hdri;
pub mod line;
pub mod material;
pub mod radiance;
pub mod texture;
pub mod tractography;
pub mod volume;
pub mod volume_fraction;
pub mod volume_mask;

use std::f32::consts::PI;

use environment::Environment;
use glam::{Mat4, UVec3};
use line::LineBuffer;
use tractography::Tractography;
use volume::PhysicalVolume;

use crate::{
    asset::{
        colormap::Colormap, crop::CropBuffer, hdri::HdriBuffer, radiance::GaussianRadianceBuffer,
        volume_fraction::VolumeFractionBuffer, volume_mask::VolumeMaskBuffer,
    },
    controller::{settings::RenderMode, Controller},
    file::FileStage,
    gpu::Gpu,
};

pub struct Asset {
    pub colormap: Colormap,
    pub environment: Environment,
    pub tractography: Tractography,
    pub hdri: HdriBuffer,
    pub crop: CropBuffer,

    pub line: Option<LineBuffer>,
    pub volumes: Vec<VolumeFractionBuffer>,
    pub masks: Vec<VolumeMaskBuffer>,
    // The single shared medium: volume density in the sampled textures (marched
    // by the volume renderer), plus a `line_extinction` side texture the line
    // deposit fills. Allocated whenever a volume OR a line is loaded.
    pub physical_volume: Option<PhysicalVolume>,
    // Primary cascade. Combined: occlusion = volume + lines, sampled by both
    // renderers. X-ray: occlusion = volume only, sampled by the volume renderer.
    pub radiance: Option<GaussianRadianceBuffer>,
    // X-ray only, and only when a volume is also present: the line-only cascade
    // the line surface shader samples so lines self-shadow without volume shadows.
    pub radiance_lines: Option<GaussianRadianceBuffer>,

    // Last `settings.volume` / `settings.index_buffer_size` baked into
    // `line`'s acceleration structure; a mismatch triggers `LineBuffer::resize`.
    volume: u32,
    index_buffer_size: u32,

    // (size, world→texture transform, radiance divisor, has_volume, has_line) the
    // `physical_volume` + `radiance` were last built for. Mode-independent so a
    // mode toggle doesn't churn them.
    physical_key: Option<(UVec3, Mat4, u32, bool, bool)>,
    // (has_volume, has_line, render_mode): drives `radiance_lines` (de)allocation
    // and the per-cascade `set_cascade_sources` calls.
    cascade_key: Option<(bool, bool, RenderMode)>,

    pub changed: bool,
}

impl Asset {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            colormap: Colormap::new(gpu),
            environment: Environment::new(gpu),
            tractography: Tractography::new(gpu),
            crop: CropBuffer::new(gpu),
            hdri: HdriBuffer::new(gpu),
            masks: vec![VolumeMaskBuffer::none(gpu)],

            line: None,
            volumes: Vec::new(),
            physical_volume: None,
            radiance: None,
            radiance_lines: None,
            volume: 0,
            index_buffer_size: 0,
            physical_key: None,
            cascade_key: None,
            changed: false,
        }
    }

    pub fn update(&mut self, gpu: &Gpu, controller: &mut Controller) {
        self.changed = false;

        FileStage::on_lines(|lines| {
            let line = LineBuffer::new(gpu, &lines, &self.colormap, controller.settings());

            self.volume = controller.settings().volume;
            self.index_buffer_size = controller.settings().index_buffer_size;

            if let Some(volume) = self
                .volumes
                .iter()
                .max_by_key(|volume| volume.size().element_product())
            {
                line.set_transform(gpu, &volume.transform());
            }

            self.line = Some(line);

            self.changed = true;
        });

        FileStage::on_track_scalars(|track_scalars| {
            if let Some(line) = &mut self.line {
                for scalar in track_scalars {
                    line.set_scalar(gpu, scalar);
                }
            }
        });

        FileStage::on_volumes(|volumes| {
            for volume in &volumes {
                if volume.name().contains("mask") {
                    self.masks.push(VolumeMaskBuffer::new(gpu, volume));
                } else {
                    self.volumes
                        .push(VolumeFractionBuffer::new(gpu, volume, &self.colormap));
                }
            }

            if let Some(volume) = self
                .volumes
                .iter()
                .max_by_key(|volume| volume.size().element_product())
            {
                if let Some(line) = &self.line {
                    line.set_transform(gpu, &volume.transform());
                }
            }

            self.changed = true;
        });

        FileStage::on_hdris(|hdris| {
            for hdri in hdris {
                self.hdri.import(gpu, hdri);
            }

            self.changed = true;
        });

        // Rotation is driven by the same shift+drag gesture as the tractography
        // light (Light::yaw) rather than its own slider, so the environment
        // rotates in lockstep with it.
        self.hdri.settings_mut().rotation = controller.light().yaw() / (2.0 * PI);
        self.hdri.update_settings(gpu);
        self.crop.update_settings(gpu);
        self.tractography.update(gpu, controller);

        if let Some(line) = &self.line {
            line.update_settings(gpu);
        }
        for volume in &self.volumes {
            volume.update_settings(gpu);
        }
        for mask in &self.masks {
            mask.update_settings(gpu);
        }

        // Rebuild the tractography acceleration structure when the voxel
        // resolution or index-buffer budget changed (this used to rebuild the
        // whole `Frame` in `Surface::maybe_resize`). Also grow the index
        // buffer when the last voxelization reported it overflowed.
        if let Some(line) = &mut self.line {
            let required = line.culling().required_index_size();
            if required > controller.settings().index_buffer_size {
                controller.settings_mut().index_buffer_size = required.next_power_of_two();
            }

            let settings = controller.settings();

            if settings.volume != self.volume
                || settings.index_buffer_size != self.index_buffer_size
            {
                line.resize(gpu, settings);
                self.volume = settings.volume;
                self.index_buffer_size = settings.index_buffer_size;
                self.changed = true;
            } else {
                // Only kick off a fresh readback on the path that isn't about
                // to tear down this CullingBuffer, mirroring the old guard in
                // `Surface::maybe_resize`.
                line.culling().refresh_required_index_size(gpu);
            }
        }

        self.sync_physical_volume(gpu, controller);
    }

    /// Keep the single shared `physical_volume` + its `radiance` cascade (and,
    /// in x-ray mode, the extra `radiance_lines` cascade) sized/oriented to the
    /// scene. Reference: the largest volume fraction if there is one, otherwise
    /// a `settings.volume`³ box around the lines so a lines-only scene still gets
    /// full cascade lighting. `physical_volume` / `radiance` realloc only when
    /// that reference changes; `radiance_lines` follows the render mode. Any
    /// (re)allocation flags `changed` so frame accumulation resets.
    fn sync_physical_volume(&mut self, gpu: &Gpu, controller: &Controller) {
        let reference = self
            .volumes
            .iter()
            .max_by_key(|volume| volume.size().element_product())
            .map(|volume| (volume.size(), volume.transform()))
            .or_else(|| {
                self.line.as_ref().map(|line| {
                    (
                        UVec3::splat(controller.settings().volume.max(1)),
                        line.bounds().transform().inverse(),
                    )
                })
            });

        let divisor = controller.radiance().resolution().max(1);
        let has_volume = !self.volumes.is_empty();
        let has_line = self.line.is_some();
        let mode = controller.settings().render_mode;

        let radiance =
            |size: UVec3| GaussianRadianceBuffer::new(gpu, (size / divisor).max(UVec3::ONE));

        let key =
            reference.map(|(size, transform)| (size, transform, divisor, has_volume, has_line));

        if key != self.physical_key {
            match reference {
                Some((size, transform)) => {
                    self.physical_volume = Some(PhysicalVolume::new(gpu, size, transform));
                    self.radiance = Some(radiance(size));
                }
                None => {
                    self.physical_volume = None;
                    self.radiance = None;
                }
            }
            self.radiance_lines = None;
            self.physical_key = key;
            self.cascade_key = None;
            self.changed = true;
        }

        let cascade_key = Some((has_volume, has_line, mode));
        if cascade_key == self.cascade_key {
            return;
        }

        // The separate line cascade only earns its keep in x-ray with both a
        // volume (to exclude) and lines present.
        let split = mode == RenderMode::XRay && has_volume && has_line;

        match (split, reference) {
            (true, Some((size, _))) if self.radiance_lines.is_none() => {
                self.radiance_lines = Some(radiance(size));
            }
            (false, _) => self.radiance_lines = None,
            _ => {}
        }

        if let Some(rad) = &self.radiance {
            let (volume, lines) = if split {
                (true, false)
            } else {
                (has_volume, has_line)
            };
            rad.set_cascade_sources(gpu, volume, lines);
        }
        if let Some(rad) = &self.radiance_lines {
            rad.set_cascade_sources(gpu, false, true);
        }

        self.cascade_key = cascade_key;
        self.changed = true;
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
