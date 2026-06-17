use std::any::type_name;

use wgpu::{
    Color, CommandEncoder, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, Texture, TextureViewDescriptor,
};

use crate::{
    gpu::Gpu,
    surface::{color::ColorBuffer, Surface},
};

pub struct CopyPipeline {
    pipeline: RenderPipeline,
}

impl CopyPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[&ColorBuffer::layout(gpu)]),
                Surface::target(),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, source: &ColorBuffer, texture: &Texture) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &texture.create_view(&TextureViewDescriptor::default()),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, source.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
