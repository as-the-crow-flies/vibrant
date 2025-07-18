use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::scalar::{R32Float, ScalarTexture3D},
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
                &gpu.pipeline_layout(&[
                    &ScalarTexture3D::<R32Float>::layout(gpu),
                    &ScalarTexture3D::<R32Float>::layout(gpu),
                    &ScalarTexture3D::<R32Float>::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                ColorBuffer::target_srgb(),
                &gpu.shader(include_str!("volume.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.occupancy().density().binding(), &[]);
        pass.set_bind_group(1, frame.occlusion().ambient().binding(), &[]);
        pass.set_bind_group(2, frame.occlusion().directional().binding(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
