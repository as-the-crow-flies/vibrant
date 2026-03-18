pub mod bloom;
pub mod depth_of_field;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    controller::settings::Settings,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{bloom::BloomBuffer, color::ColorBuffer, Frame},
};

pub struct PostProcessingPipeline {
    bloom: bloom::BloomRenderPipeline,
    depth_of_field: depth_of_field::DepthOfFieldRenderPipeline,
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
                &[Some(ColorBuffer::target_srgb())],
                &gpu.shader(include_str!("post.wgsl")),
            ),
            bloom: bloom::BloomRenderPipeline::new(gpu),
            depth_of_field: depth_of_field::DepthOfFieldRenderPipeline::new(gpu),
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
        if settings.depth_of_field {
            self.depth_of_field.dispatch(cmd, environment, frame);
        }
        self.composite(cmd, &frame, environment, settings);
    }

    fn composite(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        settings: &Settings,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.post().attachment_srgb_clear())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, environment.binding(), &[]);
        if settings.depth_of_field {
            pass.set_bind_group(1, frame.dof().read_a(), &[]);
        } else {
            pass.set_bind_group(1, frame.color().binding(), &[]);
        }
        pass.set_bind_group(2, frame.bloom().read_a(), &[]);
        pass.draw(0..6, 0..1);
    }
}
