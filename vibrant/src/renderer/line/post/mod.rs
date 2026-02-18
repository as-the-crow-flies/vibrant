pub mod bloom;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    controller::settings::Settings,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{bloom::BloomBuffer, color::ColorBuffer, Frame},
};

pub struct PostProcessingPipeline {
    bloom: bloom::BloomRenderPipeline,
    pipeline: RenderPipeline,
}

impl PostProcessingPipeline {
    pub fn new(gpu: &Gpu) -> PostProcessingPipeline {
        PostProcessingPipeline {
            pipeline: gpu.quad(
                "Post",
                &gpu.pipeline_layout(&[
                    &Environment::layout(gpu),
                    &ColorBuffer::layout(gpu),
                    &BloomBuffer::read_layout(gpu),
                ]),
                ColorBuffer::target_srgb(),
                &gpu.shader(include_str!("post.wgsl")),
            ),
            bloom: bloom::BloomRenderPipeline::new(gpu),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        settings: &Settings,
    ) {
        if settings.bloom {
            self.bloom.dispatch(cmd, environment, frame);
        }
        self.composite(cmd, &frame, environment);
    }

    fn composite(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.post().attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, frame.color().binding(), &[]);
        pass.set_bind_group(2, frame.bloom().read_a(), &[]);
        pass.draw(0..6, 0..1);
    }
}
