use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct PostProcessingPipeline {
    pipeline: RenderPipeline,
}

impl PostProcessingPipeline {
    pub fn new(gpu: &Gpu) -> PostProcessingPipeline {
        PostProcessingPipeline {
            pipeline: gpu.quad(
                "Post",
                &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
                ColorBuffer::target_srgb(),
                &gpu.shader(include_str!("post.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, frame: &Frame) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.post().attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, frame.color().binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
