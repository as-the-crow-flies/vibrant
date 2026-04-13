use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::{hdri::HdriBuffer, radiance::RadianceVolume, volume::PhysicalVolume},
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
                    &RadianceVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                ColorBuffer::target_srgb(),
                &gpu.shader(include_str!("trace.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        volume: &PhysicalVolume,
        radiance: &RadianceVolume,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.post().attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.trace);
        pass.set_bind_group(0, volume.binding_read(), &[]);
        pass.set_bind_group(1, radiance.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
