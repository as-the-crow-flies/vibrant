use wgpu::{BindGroup, CommandEncoder, RenderPassDescriptor, RenderPipeline, TextureFormat};

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
        Self::new_with_format(gpu, ColorBuffer::FORMAT)
    }

    pub fn new_with_format(gpu: &Gpu, format: TextureFormat) -> Self {
        Self {
            pipeline: gpu.quad(
                "Volume",
                &gpu.pipeline_layout(&[&Frame::layout(gpu), &Environment::layout(gpu)]),
                ColorBuffer::target_with_format(format),
                &gpu.shader(include_str!("volume.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_clear())],
            label: Some("Volume"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }

    pub fn render_to(
        &self,
        cmd: &mut CommandEncoder,
        target: &ColorBuffer,
        frame: &Frame,
        env_binding: &BindGroup,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(target.attachment_clear())],
            label: Some("Volume::Focus"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, env_binding, &[]);
        pass.draw(0..4, 0..1);
    }
}
