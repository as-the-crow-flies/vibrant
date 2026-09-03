use std::any::type_name;

use wgpu::{
    Color, CommandEncoder, ComputePassDescriptor, ComputePipeline, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp,
};

use crate::{
    gpu::Gpu,
    surface::{accumulate::AccumulateBuffer, color::ColorBuffer, Frame},
};

/// The `accumulate` and `reduce` render/compute pipelines.
pub struct AccumulatePipeline {
    accumulate: RenderPipeline,
    reduce: ComputePipeline,
}

impl AccumulatePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let accumulate = gpu.quad(
            type_name::<Self>(),
            &gpu.pipeline_layout(&[
                &ColorBuffer::layout(gpu),
                &ColorBuffer::layout(gpu),
                &AccumulateBuffer::uniform_layout(gpu),
            ]),
            ColorBuffer::target(),
            &gpu.shader(include_str!("accumulate.wgsl")),
        );

        let reduce = gpu.compute(
            type_name::<Self>(),
            &gpu.pipeline_layout(&[
                &ColorBuffer::layout(gpu),
                &ColorBuffer::layout(gpu),
                &AccumulateBuffer::metric_layout(gpu),
            ]),
            &gpu.shader(include_str!("reduce.wgsl")),
        );

        Self { accumulate, reduce }
    }

    /// Fold `frame.color()` (this frame's raw sample) into the running mean:
    /// `frame.accum(parity)` holds the mean of the earlier samples, the result
    /// is written to `frame.accum(parity ^ 1)`. The caller primes the sample
    /// count via [`AccumulateBuffer::set_sample`] first.
    pub fn accumulate(
        &self,
        cmd: &mut CommandEncoder,
        buffer: &AccumulateBuffer,
        frame: &Frame,
        parity: usize,
    ) {
        let prev = frame.accum(parity);
        let next = frame.accum(parity ^ 1);

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: next.view(),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        pass.set_pipeline(&self.accumulate);
        pass.set_bind_group(0, frame.color().binding(), &[]);
        pass.set_bind_group(1, prev.binding(), &[]);
        pass.set_bind_group(2, buffer.uniform_binding(), &[]);
        pass.draw(0..4, 0..1);
    }

    /// Sum the per-pixel luminance residual between `prev` and `next` into the
    /// metric buffer for readback via
    /// [`crate::surface::accumulate::Accumulator::read_metric`].
    pub fn reduce(
        &self,
        cmd: &mut CommandEncoder,
        buffer: &AccumulateBuffer,
        prev: &ColorBuffer,
        next: &ColorBuffer,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        pass.set_pipeline(&self.reduce);
        pass.set_bind_group(0, prev.binding(), &[]);
        pass.set_bind_group(1, next.binding(), &[]);
        pass.set_bind_group(2, buffer.metric_binding(), &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
