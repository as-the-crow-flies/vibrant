use std::any::type_name;

use wgpu::{
    CommandEncoder, FragmentState, PrimitiveState, PrimitiveTopology, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::scalar::ScalarTexture,
    gpu::Gpu,
    renderer::{constants::Constants, environment::Environment},
    surface::{color::Color, SurfaceBuffer},
};

pub struct TractogramDensityShading {
    pipeline: RenderPipeline,
}

impl TractogramDensityShading {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let shader = gpu.shader(
            &(Environment::wgsl() + include_str!("volume.wgsl")),
            Some(constants),
        );

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[
                        &ScalarTexture::layout(gpu),
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
                        targets: &[Some(Color::target_srgb())],
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
        frame: &SurfaceBuffer,
        environment: &Environment,
        scalar: &ScalarTexture,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, scalar.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
