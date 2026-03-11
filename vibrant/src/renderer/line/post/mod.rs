use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    controller::settings::Settings,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct PostProcessingPipeline {
    passthrough: RenderPipeline,
    bright: RenderPipeline,
    blur_x: RenderPipeline,
    blur_y: RenderPipeline,
    composite: RenderPipeline,
}

impl PostProcessingPipeline {
    pub fn new(gpu: &Gpu) -> PostProcessingPipeline {
        PostProcessingPipeline {
            passthrough: gpu.quad(
                "Post::Passthrough",
                &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
                ColorBuffer::target(),
                &gpu.shader(include_str!("post.wgsl")),
            ),
            bright: gpu.quad(
                "Post::Bloom::Bright",
                &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
                ColorBuffer::target(),
                &gpu.shader(include_str!("bright.wgsl")),
            ),
            blur_x: gpu.quad(
                "Post::Bloom::BlurX",
                &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
                ColorBuffer::target(),
                &gpu.shader(include_str!("blur_x.wgsl")),
            ),
            blur_y: gpu.quad(
                "Post::Bloom::BlurY",
                &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
                ColorBuffer::target(),
                &gpu.shader(include_str!("blur_y.wgsl")),
            ),
            composite: gpu.quad(
                "Post::Bloom::Composite",
                &gpu.pipeline_layout(&[
                    &Environment::layout(gpu),
                    &ColorBuffer::layout(gpu),
                    &ColorBuffer::layout(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(include_str!("composite.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        settings: &Settings,
    ) {
        if !settings.bloom_enabled {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("Post::Passthrough"),
                color_attachments: &[Some(frame.post().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.passthrough);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.color().binding(), &[]);
            pass.draw(0..6, 0..1);

            return;
        }

        // Step 1: Extract only bright pixels from scene color.
        {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("Post::Bloom::Bright"),
                color_attachments: &[Some(frame.bloom_a().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.bright);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.color().binding(), &[]);
            pass.draw(0..6, 0..1);
        }

        // Step 2: Blur horizontally into temporary ping-pong target.
        {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("Post::Bloom::BlurX"),
                color_attachments: &[Some(frame.bloom_b().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.blur_x);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.bloom_a().binding(), &[]);
            pass.draw(0..6, 0..1);
        }

        // Step 3: Blur vertically to complete separable Gaussian blur.
        {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("Post::Bloom::BlurY"),
                color_attachments: &[Some(frame.bloom_a().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.blur_y);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.bloom_b().binding(), &[]);
            pass.draw(0..6, 0..1);
        }

        // Step 4: Add blurred bloom back onto original scene and write to final post target.
        {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("Post::Bloom::Composite"),
                color_attachments: &[Some(frame.post().attachment())],
                ..Default::default()
            });

            pass.set_pipeline(&self.composite);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.color().binding(), &[]);
            pass.set_bind_group(2, frame.bloom_a().binding(), &[]);
            pass.draw(0..6, 0..1);
        }
    }
}
