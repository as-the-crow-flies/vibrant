use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct VolumeLineRenderPipeline {
    pipeline: RenderPipeline,
}

impl VolumeLineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.quad(
                "Volume",
                &gpu.pipeline_layout(&[&Frame::layout(gpu), &Environment::layout(gpu)]),
                ColorBuffer::target_srgb(),
                &gpu.shader(include_str!("volume.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb_clear())],
            label: Some("Volume"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
