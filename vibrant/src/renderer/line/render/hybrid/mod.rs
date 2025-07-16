use std::any::type_name;

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::{
        line::LineSet,
        scalar::{R32Float, R8Uint, ScalarTexture3D},
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::Color, occupancy::Occupancy, Frame},
};

pub struct HybridLineRenderPipeline {
    pipeline: RenderPipeline,
}

impl HybridLineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            pipeline: gpu.quad(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &LineSet::layout(gpu, true),
                    &Occupancy::layout(gpu, true),
                    &ScalarTexture3D::<R32Float>::layout(gpu),
                    &ScalarTexture3D::<R8Uint>::layout(gpu),
                    &ScalarTexture3D::<R32Float>::layout(gpu),
                    &ScalarTexture3D::<R32Float>::layout(gpu),
                    &ScalarTexture3D::<R32Float>::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                Color::target_srgb(),
                &gpu.shader(include_str!("hybrid.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        tractogram: &LineSet,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, tractogram.binding(true), &[]);
        pass.set_bind_group(1, frame.occupancy().binding(true), &[]);
        pass.set_bind_group(2, frame.occupancy().texture().binding(), &[]);
        pass.set_bind_group(3, frame.density().count().binding(), &[]);
        pass.set_bind_group(4, frame.density().density().binding(), &[]);
        pass.set_bind_group(5, frame.occlusion().ambient().binding(), &[]);
        pass.set_bind_group(6, frame.occlusion().directional().binding(), &[]);
        pass.set_bind_group(7, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
