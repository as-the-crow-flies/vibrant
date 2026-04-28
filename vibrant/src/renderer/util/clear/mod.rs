use std::any::type_name;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{gpu::Gpu, surface::color::ColorBuffer};

pub struct ClearPipeline {
    pipeline: RenderPipeline,
}

impl ClearPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[]),
                ColorBuffer::target_srgb(),
                &gpu.shader(include_str!("clear.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, color: &ColorBuffer) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(color.attachment_srgb_clear())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.draw(0..4, 0..1);
    }
}
