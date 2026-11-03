use std::any::type_name;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    gpu::{Gpu, Lazy},
    surface::color::ColorBuffer,
};

pub struct ClearPipeline {
    pipeline: Lazy<RenderPipeline>,
}

impl ClearPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.lazy(type_name::<Self>(), |gpu| {
                gpu.quad(
                    type_name::<Self>(),
                    &gpu.pipeline_layout(&[]),
                    ColorBuffer::target(),
                    &gpu.shader("renderer/util/clear/clear.wgsl"),
                )
            }),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, color: &ColorBuffer) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(color.attachment_clear())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.draw(0..4, 0..1);
    }
}
