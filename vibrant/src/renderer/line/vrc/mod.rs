use wgpu::{CommandEncoder, ComputePipeline};

use crate::{
    asset::line::LineSet, gpu::Gpu, renderer::environment::Environment, sort::KeyValuePair,
};

pub struct VrcVoxelizationPipeline {
    quantize: ComputePipeline,
}

impl VrcVoxelizationPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let quantize = gpu.compute(
            "VrcVoxelizationPipeline::Quantize",
            &gpu.pipeline_layout(&[
                &LineSet::layout(gpu, true),
                &KeyValuePair::layout(gpu),
                &Environment::layout(gpu),
            ]),
            &gpu.shader(include_str!("quantize.wgsl")),
        );

        Self { quantize }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, line: &LineSet) {}
}
