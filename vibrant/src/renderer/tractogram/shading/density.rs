use std::any::type_name;

use wgpu::{
    CommandEncoder, FragmentState, PrimitiveState, PrimitiveTopology, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::scalar::{R8Unorm, ScalarTexture3D},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::Color, Frame},
};

pub struct VolumeShadingPipeline {
    pipeline: RenderPipeline,
}

impl VolumeShadingPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let shader = gpu.shader(include_str!("density.wgsl"));

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[
                        &ScalarTexture3D::<R8Unorm>::layout(gpu),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: &shader,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    fragment: Some(FragmentState {
                        module: &shader,
                        entry_point: Some("fragment"),
                        targets: &[Some(Color::target())],
                        compilation_options: Default::default(),
                    }),
                    multisample: Default::default(),
                    depth_stencil: None,
                    multiview: None,
                    cache: None,
                }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        texture: &ScalarTexture3D<R8Unorm>,
        environment: &Environment,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, texture.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
