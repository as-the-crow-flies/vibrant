use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, depth::DepthBuffer, dof::DofBuffer, Frame},
};

pub struct DepthOfFieldRenderPipeline {
    blur_h: ComputePipeline,
    blur_v: ComputePipeline,
}

impl DepthOfFieldRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let preamble = include_str!("blur.wgsl");
        let kernel = include_str!("../blur.wgsl");
        let blur_h_source = format!("const DIRECTION = vec2<f32>(1.0, 0.0);\n{preamble}{kernel}");
        let blur_v_source = format!("const DIRECTION = vec2<f32>(0.0, 1.0);\n{preamble}{kernel}");

        let blur_layout = gpu.pipeline_layout(&[
            &Environment::layout(gpu),
            &DofBuffer::read_layout(gpu),
            &DofBuffer::write_layout(gpu),
            &DepthBuffer::layout(gpu),
        ]);

        let first_pass_layout = gpu.pipeline_layout(&[
            &Environment::layout(gpu),
            &ColorBuffer::layout(gpu),
            &DofBuffer::write_layout(gpu),
            &DepthBuffer::layout(gpu),
        ]);

        Self {
            blur_h: gpu.compute(
                "DoF::Blur::Horizontal",
                &first_pass_layout,
                &gpu.shader(&blur_h_source),
            ),
            blur_v: gpu.compute(
                "DoF::Blur::Vertical",
                &blur_layout,
                &gpu.shader(&blur_v_source),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, frame: &Frame) {
        let w = frame.dof().width().div_ceil(4);
        let h = frame.dof().height().div_ceil(4);

        // H pass: color → dof_b
        {
            let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
                label: Some("DoF::Blur::Horizontal"),
                ..Default::default()
            });

            pass.set_pipeline(&self.blur_h);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.color().binding(), &[]);
            pass.set_bind_group(2, frame.dof().write_b(), &[]);
            pass.set_bind_group(3, frame.depth().binding(), &[]);
            pass.dispatch_workgroups(w, h, 1);
        }

        // V pass: dof_b → dof_a
        {
            let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
                label: Some("DoF::Blur::Vertical"),
                ..Default::default()
            });

            pass.set_pipeline(&self.blur_v);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.dof().read_b(), &[]);
            pass.set_bind_group(2, frame.dof().write_a(), &[]);
            pass.set_bind_group(3, frame.depth().binding(), &[]);
            pass.dispatch_workgroups(w, h, 1);
        }
    }
}
