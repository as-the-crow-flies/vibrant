use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::volume::PhysicalVolume,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct AnatomyTracePipeline {
    trace: RenderPipeline,
}

impl AnatomyTracePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            trace: gpu.quad(
                "AnatomyTrace",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(include_str!("trace.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        volume: &PhysicalVolume,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.post().attachment_clear())],
            ..Default::default()
        });

        pass.set_pipeline(&self.trace);
        pass.set_bind_group(0, volume.binding_read(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
