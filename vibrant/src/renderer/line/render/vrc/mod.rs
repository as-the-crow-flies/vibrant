use std::any::type_name;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::line::LineSet,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::{environment::Environment, wgsl::TRACE},
    sort::KeyValuePair,
    surface::{color::ColorBuffer, vrc::VrcBuffer, Frame},
};

pub struct VrcLineRenderPipeline {
    opaque: RenderPipeline,
    transparent: RenderPipeline,
}

impl VrcLineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = &gpu.pipeline_layout(&[
            &Frame::layout(gpu),
            &Environment::layout(gpu),
            &KeyValuePair::layout(gpu),
            &VrcBuffer::layout(gpu),
        ]);

        Self {
            opaque: gpu.quad(
                type_name::<Self>(),
                &layout,
                ColorBuffer::target_srgb(),
                &gpu.shader(&(TRACE.to_string() + include_str!("opaque.wgsl"))),
            ),
            transparent: gpu.quad(
                type_name::<Self>(),
                &layout,
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
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb_clear())],
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        if settings.alpha == 1.0 {
            pass.set_pipeline(&self.opaque);
        } else {
            pass.set_pipeline(&self.transparent);
        }

        pass.set_bind_group(0, frame.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(2, line.vrc().ping().binding(), &[]);
        pass.set_bind_group(3, frame.vrc().binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
