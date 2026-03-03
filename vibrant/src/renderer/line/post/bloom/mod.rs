use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{bloom::BloomBuffer, color::ColorBuffer, Frame},
};

pub struct BloomRenderPipeline {
    extract: ComputePipeline,
    blur_h: ComputePipeline,
    blur_v: ComputePipeline,
}

impl BloomRenderPipeline {
    pub fn new(gpu: &Gpu) -> BloomRenderPipeline {
        let preamble = include_str!("blur.wgsl");
        let kernel = include_str!("../blur.wgsl");
        let blur_h_source = format!("const DIRECTION = vec2<f32>(1.0, 0.0);\n{preamble}{kernel}");
        let blur_v_source = format!("const DIRECTION = vec2<f32>(0.0, 1.0);\n{preamble}{kernel}");

        let blur_layout = gpu.pipeline_layout(&[
            &Environment::layout(gpu),
            &BloomBuffer::read_layout(gpu),
            &BloomBuffer::write_layout(gpu),
        ]);

        BloomRenderPipeline {
            extract: gpu.compute(
                "Bloom::Extract",
                &gpu.pipeline_layout(&[
                    &Environment::layout(gpu),
                    &ColorBuffer::layout(gpu),
                    &BloomBuffer::write_layout(gpu),
                ]),
                &gpu.shader(include_str!("extract.wgsl")),
            ),
            blur_h: gpu.compute(
                "Bloom::Blur::Horizontal",
                &blur_layout,
                &gpu.shader(&blur_h_source),
            ),
            blur_v: gpu.compute(
                "Bloom::Blur::Vertical",
                &blur_layout,
                &gpu.shader(&blur_v_source),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, frame: &Frame) {
        let bloom = frame.bloom();
        let w = bloom.width().div_ceil(4);
        let h = bloom.height().div_ceil(4);

        self.extract(cmd, environment, frame, w, h);

        // two iterations of separable blur for a wider, softer glow
        for _ in 0..3 {
            self.blur_h(cmd, environment, bloom, w, h);
            self.blur_v(cmd, environment, bloom, w, h);
        }
    }

    fn extract(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        w: u32,
        h: u32,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Bloom::Extract"),
            ..Default::default()
        });

        pass.set_pipeline(&self.extract);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, frame.color().binding(), &[]);
        pass.set_bind_group(2, frame.bloom().write_a(), &[]);
        pass.dispatch_workgroups(w, h, 1);
    }

    fn blur_h(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        bloom: &BloomBuffer,
        w: u32,
        h: u32,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Bloom::Blur::Horizontal"),
            ..Default::default()
        });

        pass.set_pipeline(&self.blur_h);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, bloom.read_a(), &[]);
        pass.set_bind_group(2, bloom.write_b(), &[]);
        pass.dispatch_workgroups(w, h, 1);
    }

    fn blur_v(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        bloom: &BloomBuffer,
        w: u32,
        h: u32,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Bloom::Blur::Vertical"),
            ..Default::default()
        });

        pass.set_pipeline(&self.blur_v);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, bloom.read_b(), &[]);
        pass.set_bind_group(2, bloom.write_a(), &[]);
        pass.dispatch_workgroups(w, h, 1);
    }
}
