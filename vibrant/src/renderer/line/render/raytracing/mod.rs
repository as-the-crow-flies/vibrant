use std::any::type_name;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use wgpu::BindGroup;

use crate::{
    asset::line::LineBuffer,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::{environment::Environment, wgsl::TRACE},
    surface::{color::ColorBuffer, culling::CullingBuffer, Frame},
};

pub struct RayTracingLineRenderPipeline {
    opaque: RenderPipeline,
    transparent: RenderPipeline,
}

impl RayTracingLineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            opaque: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                    &LineBuffer::layout(gpu, true),
                    &CullingBuffer::layout_read(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(&(TRACE.to_string() + include_str!("opaque.wgsl"))),
            ),
            transparent: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                    &LineBuffer::layout(gpu, true),
                    &CullingBuffer::layout_read(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(&(TRACE.to_string() + include_str!("transparent.wgsl"))),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        settings: &Settings,
        line: &LineBuffer,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_clear())],
            label: Some("Ray"),
            ..Default::default()
        });

        if settings.alpha == 1.0 {
            pass.set_pipeline(&self.opaque);
        } else {
            pass.set_pipeline(&self.transparent);
        }

        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(2, line.binding(true), &[]);
        pass.set_bind_group(3, frame.culling().binding_read(), &[]);
        pass.draw(0..4, 0..1);
    }

    pub fn render_to(
        &self,
        cmd: &mut CommandEncoder,
        target: &ColorBuffer,
        frame: &Frame,
        env_binding: &BindGroup,
        settings: &Settings,
        line: &LineBuffer,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(target.attachment_clear())],
            label: Some("Ray::Focus"),
            ..Default::default()
        });

        if settings.alpha == 1.0 {
            pass.set_pipeline(&self.opaque);
        } else {
            pass.set_pipeline(&self.transparent);
        }

        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, env_binding, &[]);
        pass.set_bind_group(2, line.binding(true), &[]);
        pass.set_bind_group(3, frame.culling().binding_read(), &[]);
        pass.draw(0..4, 0..1);
    }
}
