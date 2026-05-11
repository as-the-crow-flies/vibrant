use std::any::type_name;

use egui::Rect;
use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::line::LineBuffer,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, culling::CullingBuffer, Frame},
};

pub struct LineRenderPipeline {
    opaque: RenderPipeline,
    transparent: RenderPipeline,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let trace = include_str!("trace.wgsl");

        Self {
            opaque: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                    &LineBuffer::layout_render(gpu),
                    &CullingBuffer::layout_read(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(&(trace.to_string() + include_str!("opaque.wgsl"))),
            ),
            transparent: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                    &LineBuffer::layout_render(gpu),
                    &CullingBuffer::layout_read(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(&(trace.to_string() + include_str!("transparent.wgsl"))),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        settings: &Settings,
        viewport: Rect,
        line: &LineBuffer,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_clear())],
            label: Some("Ray"),
            ..Default::default()
        });

        pass.set_viewport(
            viewport.min.x,
            viewport.min.y,
            viewport.width(),
            viewport.height(),
            0.0,
            1.0,
        );

        if settings.alpha == 1.0 {
            pass.set_pipeline(&self.opaque);
        } else {
            pass.set_pipeline(&self.transparent);
        }

        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(2, line.binding_render(), &[]);
        pass.set_bind_group(3, frame.culling().binding_read(), &[]);
        pass.draw(0..4, 0..1);
    }
}
