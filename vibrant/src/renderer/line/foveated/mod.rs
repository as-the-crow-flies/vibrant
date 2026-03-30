use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline, TextureFormat};

use crate::{
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct FoveatedCompositePipeline {
    pipeline: RenderPipeline,
}

impl FoveatedCompositePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self::new_with_format(gpu, ColorBuffer::FORMAT)
    }

    pub fn new_with_format(gpu: &Gpu, format: TextureFormat) -> Self {
        Self {
            pipeline: gpu.quad(
                "Foveated::Composite",
                &gpu.pipeline_layout(&[
                    &Environment::layout(gpu),
                    &ColorBuffer::layout(gpu),
                    &ColorBuffer::layout(gpu),
                ]),
                ColorBuffer::target_with_format(format),
                &gpu.shader(include_str!("composite.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
    ) {
        let (Some(peripheral), Some(focus)) =
            (frame.foveated_peripheral(), frame.foveated_focus())
        else {
            return;
        };

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_clear())],
            label: Some("Foveated::Composite"),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, peripheral.binding(), &[]);
        pass.set_bind_group(2, focus.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
