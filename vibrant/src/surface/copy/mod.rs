use std::any::type_name;

use wgpu::{
    Color, ColorTargetState, ColorWrites, CommandEncoder, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp, Texture,
    TextureViewDescriptor,
};

use crate::{
    gpu::Gpu,
    surface::{color::ColorBuffer, Frame, Surface},
};

pub struct ColorCopyPipeline {
    pipeline: RenderPipeline,
}

impl ColorCopyPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[&ColorBuffer::layout(gpu)]),
                ColorTargetState {
                    format: Surface::FORMAT,
                    blend: None,
                    write_mask: ColorWrites::all(),
                },
                &gpu.shader(include_str!("copy.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame, target: &Texture) {
        let attachment = RenderPassColorAttachment {
            view: &target.create_view(&TextureViewDescriptor::default()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        };

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(attachment)],
            label: Some("Volume"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.color().binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
