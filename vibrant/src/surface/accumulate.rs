//! Progressive frame accumulation state, owned by [`super::Surface`].
//!
//! While the 3D scene is unchanged every frame is one noisy *sample*. The
//! accumulate pass (see [`crate::renderer::accumulate::AccumulatePipeline`])
//! folds each sample into a persistent running mean held in the two
//! [`super::Frame`] `accum` buffers, and the reduce pass sums a per-pixel
//! luminance residual into [`AccumulateBuffer`]'s metric buffer for readback.
//! [`Accumulator`] counts samples and decides when the image has converged, at
//! which point the render loop can stop entirely until something changes.

use std::any::type_name;
use std::cell::Cell;

use bytemuck::{bytes_of, Pod, Zeroable};
use glam::Vec2;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    ShaderStages,
};

use crate::{
    controller::settings::Settings,
    gpu::{readback::Readback, Gpu},
};

/// Start checking the convergence metric only once this many samples are in, so
/// an early lucky reading can't stop accumulation prematurely.
const MIN_SAMPLES: u32 = 32;
/// Run the (blocking, on native) metric readback only every Nth sample.
const METRIC_INTERVAL: u32 = 16;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AccumulateUniform {
    sample: u32,
    _pad: [u32; 3],
}

/// GPU-side resources for accumulation: the per-frame `sample` uniform bound by
/// the accumulate pass, and the convergence-metric storage buffer written by
/// the reduce pass and read back on the CPU. Resolution independent - created
/// once and owned by [`super::Surface`], following the same shape as
/// [`super::color::ColorBuffer`] (`new` + static `*_layout` + `*_binding`).
pub struct AccumulateBuffer {
    uniform: Buffer,
    uniform_binding: BindGroup,
    metric: Buffer,
    metric_binding: BindGroup,
}

impl AccumulateBuffer {
    /// `residual: f32, luma: f32, count: f32, _pad: f32`.
    const METRIC_SIZE: u64 = 16;

    pub fn new(gpu: &Gpu) -> Self {
        let uniform = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: std::mem::size_of::<AccumulateUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::uniform_layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let metric = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: Self::METRIC_SIZE,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let metric_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::metric_layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: metric.as_entire_binding(),
            }],
        });

        Self {
            uniform,
            uniform_binding,
            metric,
            metric_binding,
        }
    }

    /// Layout for the accumulate pass's `sample` uniform (fragment stage).
    pub fn uniform_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }

    /// Layout for the reduce pass's metric storage buffer (compute stage).
    pub fn metric_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }

    pub fn uniform_binding(&self) -> &BindGroup {
        &self.uniform_binding
    }

    pub fn metric_binding(&self) -> &BindGroup {
        &self.metric_binding
    }

    /// The metric storage buffer, for CPU readback.
    pub fn metric(&self) -> &Buffer {
        &self.metric
    }

    /// Upload the sample index the accumulate pass should blend at.
    pub fn set_sample(&self, gpu: &Gpu, sample: u32) {
        gpu.queue().write_buffer(
            &self.uniform,
            0,
            bytes_of(&AccumulateUniform {
                sample,
                _pad: [0; 3],
            }),
        );
    }
}

/// A snapshot of accumulation progress, handed to the UI for display.
#[derive(Debug, Clone, Copy)]
pub struct AccumulationStatus {
    /// Samples folded into the current image since the scene last changed.
    pub samples: u32,
    /// Whether accumulation has stopped (sample cap or noise threshold reached).
    pub converged: bool,
}

/// What [`Accumulator::plan`] tells the renderer to do this frame.
#[derive(Debug, Clone, Copy)]
pub struct AccumulationPlan {
    /// Whether the scene should be (re)traced this frame.
    pub render: bool,
    /// Sub-pixel camera offset (pixels) to bake into the projection matrix for
    /// temporal anti-aliasing. `Vec2::ZERO` when not accumulating / not tracing.
    pub jitter: Vec2,
}

/// Sample counter + convergence state for one accumulation run. All mutation
/// goes through `Cell` so the whole thing is drivable behind `&Surface`, the
/// same way render pipelines carry per-frame state (see
/// `GaussianVolumeRenderer::active`).
pub struct Accumulator {
    samples: Cell<u32>,
    /// Index in [`super::Frame`]'s `accum` array of the buffer holding the
    /// latest running mean.
    parity: Cell<usize>,
    converged: Cell<bool>,
    metric: Readback<[f32; 4]>,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Accumulator {
    pub fn new() -> Self {
        Self {
            samples: Cell::new(0),
            parity: Cell::new(0),
            converged: Cell::new(false),
            metric: Readback::new(),
        }
    }

    /// Discard the accumulated image and start over (scene changed / resized).
    pub fn reset(&self) {
        self.samples.set(0);
        self.converged.set(false);
    }

    pub fn samples(&self) -> u32 {
        self.samples.get()
    }

    pub fn converged(&self) -> bool {
        self.converged.get()
    }

    /// Index of the [`super::Frame`] `accum` buffer holding the latest mean.
    pub fn parity(&self) -> usize {
        self.parity.get()
    }

    /// Progress snapshot for the UI.
    pub fn status(&self) -> AccumulationStatus {
        AccumulationStatus {
            samples: self.samples.get(),
            converged: self.converged.get(),
        }
    }

    /// Decide whether to trace this frame and with what sub-pixel jitter,
    /// resetting first when `dirty`.
    pub fn plan(&self, settings: &Settings, dirty: bool) -> AccumulationPlan {
        if dirty {
            self.reset();
        }

        let render = !settings.accumulate
            || (!self.converged.get() && self.samples.get() < settings.max_samples.max(1));

        let jitter = if render && settings.accumulate {
            let n = self.samples.get() + 1;
            Vec2::new(halton(n, 2) - 0.5, halton(n, 3) - 0.5)
        } else {
            Vec2::ZERO
        };

        AccumulationPlan { render, jitter }
    }

    /// Whether the `reduce` pass is worth recording for the sample about to be
    /// accumulated - i.e. its result will actually be read back. Lets the
    /// caller skip the (single-workgroup, whole-image) reduction otherwise.
    pub fn wants_metric(&self) -> bool {
        let next = self.samples.get() + 1;
        next >= MIN_SAMPLES && next.is_multiple_of(METRIC_INTERVAL)
    }

    /// Record that a sample was accumulated this frame.
    pub fn advance(&self, settings: &Settings) {
        self.samples.set(self.samples.get() + 1);
        self.parity.set(self.parity.get() ^ 1);

        if self.samples.get() >= settings.max_samples.max(1) {
            self.converged.set(true);
        }
    }

    /// Whether the render loop should keep drawing: `true` while still refining
    /// (or in legacy continuous mode), `false` once converged.
    pub fn accumulating(&self, settings: &Settings) -> bool {
        !settings.accumulate || !self.converged.get()
    }

    /// Kick off a metric readback every `METRIC_INTERVAL` samples (once past
    /// `MIN_SAMPLES`), and fold any completed reading into `converged`.
    pub fn read_metric(&self, gpu: &Gpu, buffer: &AccumulateBuffer, settings: &Settings) {
        if self.converged.get() || self.samples.get() < MIN_SAMPLES {
            return;
        }

        if self.samples.get().is_multiple_of(METRIC_INTERVAL) {
            let gpu = gpu.clone();
            let metric = buffer.metric().clone();
            self.metric.refresh(async move {
                let raw = gpu.read_buffer::<f32>(&metric).await?;
                raw.get(0..4).map(|v| [v[0], v[1], v[2], v[3]])
            });
        }

        if let Some([residual, luma, _, _]) = self.metric.get() {
            if luma > 1.0e-6 && residual / luma < settings.noise_threshold {
                self.converged.set(true);
            }
        }
    }
}

/// `index`-th point of the base-`base` Halton low-discrepancy sequence, in
/// `[0, 1)`.
fn halton(mut index: u32, base: u32) -> f32 {
    let mut f = 1.0_f32;
    let mut result = 0.0_f32;

    while index > 0 {
        f /= base as f32;
        result += f * (index % base) as f32;
        index /= base;
    }

    result
}
