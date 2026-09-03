pub mod accumulate;
pub mod color;

use accumulate::{AccumulateBuffer, AccumulationPlan, AccumulationStatus, Accumulator};
use color::ColorBuffer;
use wgpu::{
    CommandEncoder, CompositeAlphaMode, CurrentSurfaceTexture, PresentMode, SurfaceColorSpace,
    SurfaceColorSpaces, SurfaceConfiguration, SurfaceTarget, SurfaceTexture, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

use crate::{
    controller::settings::Settings,
    renderer::{
        accumulate::AccumulatePipeline,
        present::{PresentPipeline, Presentation},
    },
};

use super::gpu::Gpu;

pub struct Frame {
    color: ColorBuffer,
    line_depth: ColorBuffer,
    accum: [ColorBuffer; 2],
    overlay: ColorBuffer,
    export: ColorBuffer,
}

impl Frame {
    pub fn new(gpu: &Gpu, settings: &Settings) -> Self {
        Self {
            color: ColorBuffer::new(gpu, settings.width, settings.height),
            line_depth: ColorBuffer::depth(gpu, settings.width, settings.height),
            accum: [
                ColorBuffer::new(gpu, settings.width, settings.height),
                ColorBuffer::new(gpu, settings.width, settings.height),
            ],
            overlay: ColorBuffer::overlay(gpu, settings.width, settings.height),
            export: ColorBuffer::export(gpu, settings.width, settings.height),
        }
    }

    /// Linear HDR scene buffer - every renderer writes un-tone-mapped radiance here.
    pub fn color(&self) -> &ColorBuffer {
        &self.color
    }

    /// Combined-mode line-depth target: the opaque line pass writes the near→far
    /// fraction `s` of its first hit here, and the volume tracer clamps its far
    /// `t` to it. Cleared to `1.0` when no line is drawn.
    pub fn line_depth(&self) -> &ColorBuffer {
        &self.line_depth
    }

    /// One of the two ping-ponged accumulation buffers (running mean of all
    /// samples since the scene last changed). `parity` selects which; see
    /// [`accumulate::Accumulator`].
    pub fn accum(&self, parity: usize) -> &ColorBuffer {
        &self.accum[parity]
    }

    /// egui paints here; the present pass composites it over the tone-mapped scene.
    pub fn overlay(&self) -> &ColorBuffer {
        &self.overlay
    }

    /// SDR copy of the composited image, kept `COPY_SRC` for screenshot readback.
    pub fn export(&self) -> &ColorBuffer {
        &self.export
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    frame: Option<Frame>,
    present: PresentPipeline,
    /// Accumulate/reduce pipelines + their resolution-independent buffers, and
    /// the sample counter / convergence state they drive.
    accumulate: AccumulatePipeline,
    accumulate_buffer: AccumulateBuffer,
    accumulator: Accumulator,
    changed: bool,
    /// The HDR presentation to use when the toggle is on, or `None` when the
    /// surface can't present HDR at all. `ExtendedSrgbLinear` (native) is
    /// preferred over `ExtendedSrgb` (the only one the browser exposes).
    hdr: Option<Presentation>,
    presentation: Presentation,
    /// User-requested HDR peak (`Settings::hdr_headroom`), stashed each frame in
    /// `maybe_reconfigure`; clamped to the live display limit in `present`.
    hdr_headroom: f32,
}

impl Surface {
    const SDR_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
    const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let spaces = surface
            .get_capabilities(gpu.adapter())
            .color_spaces(Self::HDR_FORMAT);

        let hdr = if spaces.contains(SurfaceColorSpaces::EXTENDED_SRGB_LINEAR) {
            Some(Presentation::HdrLinear)
        } else if spaces.contains(SurfaceColorSpaces::EXTENDED_SRGB) {
            Some(Presentation::HdrEncoded)
        } else {
            None
        };

        surface.configure(gpu.device(), &Self::config(1, 1, Presentation::Sdr));

        Self {
            surface,
            frame: None,
            changed: true,
            present: PresentPipeline::new(gpu, Self::SDR_FORMAT, Self::HDR_FORMAT),
            accumulate: AccumulatePipeline::new(gpu),
            accumulate_buffer: AccumulateBuffer::new(gpu),
            accumulator: Accumulator::new(),
            hdr,
            presentation: Presentation::Sdr,
            hdr_headroom: 2.0,
        }
    }

    pub fn maybe_reconfigure(&mut self, gpu: &Gpu, settings: &Settings) {
        self.hdr_headroom = settings.hdr_headroom;

        let presentation = match (settings.hdr, self.hdr) {
            (true, Some(hdr)) => hdr,
            _ => Presentation::Sdr,
        };

        if let Some(frame) = &self.frame {
            if settings.width == frame.color().width()
                && settings.height == frame.color().height()
                && presentation == self.presentation
            {
                self.changed = false;
                return;
            }
        }

        self.frame.take();
        self.frame = Some(Frame::new(gpu, settings));
        self.presentation = presentation;

        self.surface.configure(
            gpu.device(),
            &Self::config(settings.width, settings.height, presentation),
        );

        self.changed = true;
    }

    fn get_current_texture(&self) -> Option<SurfaceTexture> {
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => Some(texture),
            CurrentSurfaceTexture::Suboptimal(texture) => Some(texture),
            _ => None,
        }
    }

    /// Records the SDR present into `frame.export()` so a screenshot captures
    /// exactly the on-screen SDR look, UI included. `scene` is the tone-map
    /// input (the accumulated mean, or the raw sample buffer).
    pub fn export(&self, gpu: &Gpu, cmd: &mut CommandEncoder, frame: &Frame, scene: &ColorBuffer) {
        self.present
            .export(gpu, cmd, scene, frame.overlay(), frame.export().view());
    }

    pub fn present(&self, gpu: &Gpu, mut cmd: CommandEncoder, scene: &ColorBuffer) {
        if let (Some(frame), Some(surface)) = (&self.frame, self.get_current_texture()) {
            let view = surface
                .texture
                .create_view(&TextureViewDescriptor::default());

            let headroom = if self.presentation == Presentation::Sdr {
                1.0
            } else {
                self.hdr_headroom.clamp(1.0, self.hdr_headroom_limit(gpu))
            };

            self.present.dispatch(
                gpu,
                &mut cmd,
                scene,
                frame.overlay(),
                &view,
                self.presentation,
                headroom,
            );

            gpu.submit(cmd);
            gpu.queue().present(surface);
        }
    }

    /// Progress snapshot for the UI.
    pub fn accumulation(&self) -> AccumulationStatus {
        self.accumulator.status()
    }

    /// Decide whether the renderer should trace the scene this frame and with
    /// what sub-pixel jitter. `scene_dirty` = something the surface can't see
    /// changed (assets, camera, lighting); the surface folds in its own
    /// resize/reconfigure `changed` flag.
    pub fn plan_accumulation(&self, settings: &Settings, scene_dirty: bool) -> AccumulationPlan {
        self.accumulator
            .plan(settings, scene_dirty || self.changed)
    }

    /// After the renderer has traced into `frame.color()` (when `rendered`),
    /// fold that sample into the running mean and return the buffer the
    /// present/export passes should tone map: the accumulated mean, the raw
    /// sample (accumulation off), or the last mean (converged, `!rendered`).
    pub fn resolve(
        &self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        settings: &Settings,
        rendered: bool,
    ) -> Option<&ColorBuffer> {
        let frame = self.frame.as_ref()?;

        if !rendered {
            return Some(frame.accum(self.accumulator.parity()));
        }

        if !settings.accumulate {
            return Some(frame.color());
        }

        let parity = self.accumulator.parity();
        let prev = frame.accum(parity);
        let next = frame.accum(parity ^ 1);

        self.accumulate.accumulate(
            gpu,
            cmd,
            &self.accumulate_buffer,
            frame.color(),
            prev,
            next,
            self.accumulator.samples(),
        );

        // Only run the whole-image reduction on frames whose result is read back.
        if self.accumulator.wants_metric() {
            self.accumulate
                .reduce(cmd, &self.accumulate_buffer, prev, next);
        }

        self.accumulator.advance(settings);

        Some(next)
    }

    /// Fold any finished convergence reading in and, on the interval, kick off
    /// the next readback. Call after `present` has submitted the `reduce` pass.
    pub fn read_metric(&self, gpu: &Gpu, settings: &Settings) {
        self.accumulator
            .read_metric(gpu, &self.accumulate_buffer, settings);
    }

    /// Whether the render loop should schedule another frame immediately
    /// (still refining, or legacy continuous mode).
    pub fn accumulating(&self, settings: &Settings) -> bool {
        self.accumulator.accumulating(settings)
    }

    fn config(width: u32, height: u32, presentation: Presentation) -> SurfaceConfiguration {
        let (format, color_space) = match presentation {
            Presentation::Sdr => (Self::SDR_FORMAT, SurfaceColorSpace::Auto),
            Presentation::HdrLinear => (Self::HDR_FORMAT, SurfaceColorSpace::ExtendedSrgbLinear),
            Presentation::HdrEncoded => (Self::HDR_FORMAT, SurfaceColorSpace::ExtendedSrgb),
        };

        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![format],
            color_space,
        }
    }

    pub fn frame(&self) -> &Option<Frame> {
        &self.frame
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Whether the current surface/adapter can present an HDR swapchain.
    pub fn hdr_supported(&self) -> bool {
        self.hdr.is_some()
    }

    /// The largest HDR peak (multiple of SDR white) the display can drive.
    /// Prefers the ideal-conditions ceiling (Apple's `potential` EDR headroom)
    /// so the slider doesn't jitter with the OS brightness slider the way the
    /// live `current` figure does; falls back to `current`, then the
    /// nits-derived ratio, then 2.0 when the platform reports nothing (e.g. the
    /// web). Capped at 8.0. This is the upper bound the HDR strength slider and
    /// `present` clamp `Settings::hdr_headroom` to.
    pub fn hdr_headroom_limit(&self, gpu: &Gpu) -> f32 {
        let info = self.surface.display_hdr_info(gpu.adapter());

        info.headroom
            .and_then(|h| h.potential.or(h.current))
            .or_else(|| info.tone_map_headroom())
            .filter(|h| h.is_finite() && *h > 1.0)
            .unwrap_or(2.0)
            .clamp(1.0, 8.0)
    }
}
