pub mod populate;

use std::any::type_name;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::line::LineSet,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::{
        environment::Environment, line::render::raytracing::populate::LinePopulatePipeline,
        wgsl::TRACE,
    },
    surface::{color::ColorBuffer, culling::CullingBuffer, Frame},
};

pub struct RayTracingLineRenderPipeline {
    populate: LinePopulatePipeline,
    opaque: RenderPipeline,
    transparent: RenderPipeline,
}

impl RayTracingLineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            populate: LinePopulatePipeline::new(gpu),
            opaque: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                    &LineSet::layout(gpu, true),
                    &CullingBuffer::layout_read(gpu),
                ]),
                ColorBuffer::target_srgb(),
                &gpu.shader(&(TRACE.to_string() + include_str!("opaque.wgsl"))),
            ),
            transparent: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                    &LineSet::layout(gpu, true),
                    &CullingBuffer::layout_read(gpu),
                ]),
                ColorBuffer::target_srgb(),
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
        line: &LineSet,
    ) {
        self.populate
            .dispatch(cmd, frame, environment, settings, line);

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb_clear())],
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
}
