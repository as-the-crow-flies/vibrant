use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct BloomRenderPipeline {
    extract: ComputePipeline,
    blur: ComputePipeline,
}

impl BloomRenderPipeline {
    pub fn new(gpu: &Gpu) -> BloomRenderPipeline {
        BloomRenderPipeline {
            extract: gpu.compute(
                "Bloom::Extract",
                &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
                &gpu.shader(include_str!("extract.wgsl")),
            ),
            blur: gpu.compute(
                "Bloom::Blur",
                &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
                &gpu.shader(include_str!("blur.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, frame: &Frame) {
        self.extract(cmd, &frame, environment);
        self.blur(cmd, &frame, environment);
    }

    fn extract(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Bloom::Extract"),
            ..Default::default()
        });

        // let n = frame.occlusion().ambient().resolution().div_ceil(4);
        // pass.set_bind_group(0, environment.binding(), &[]);
        // pass.set_bind_group(1, frame.color().binding(), &[]);
        let w = frame.color().width().div_ceil(16);
        let h = frame.color().height().div_ceil(16);
        pass.dispatch_workgroups(w, h, 1);
    }

    fn blur(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Bloom::Blur"),
            ..Default::default()
        });

        // pass.set_bind_group(0, environment.binding(), &[]);
        // pass.set_bind_group(1, frame.color().binding(), &[]);
        let w = frame.color().width().div_ceil(16);
        let h = frame.color().height().div_ceil(16);
        pass.dispatch_workgroups(w, h, 1);
    }
}
