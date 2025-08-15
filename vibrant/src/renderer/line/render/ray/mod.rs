use std::any::type_name;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::line::LineSet,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, culling::CullingBuffer, Frame},
};

pub struct RayCastingLineRenderPipeline {
    pipeline: RenderPipeline,
}

impl RayCastingLineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &LineSet::layout(gpu, true),
                    &CullingBuffer::layout_read(gpu),
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                ColorBuffer::target_srgb(),
                &gpu.shader(include_str!("ray.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb_clear())],
            label: Some("Ray"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, line.binding(true), &[]);
        pass.set_bind_group(1, frame.culling().binding_read(), &[]);
        pass.set_bind_group(2, frame.binding(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
