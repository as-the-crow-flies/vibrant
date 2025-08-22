use wgpu::ComputePipeline;

use crate::{asset::line::LineSet, gpu::Gpu, renderer::environment::Environment};

pub struct SegmentPipeline {
    segment: ComputePipeline,
}

impl SegmentPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            segment: gpu.compute(
                "Segment::Segment",
                &gpu.pipeline_layout(&[
                    &LineSet::layout_raw(gpu),
                    &LineSet::layout(gpu, false),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("segment.wgsl")),
            ),
        }
    }
}
