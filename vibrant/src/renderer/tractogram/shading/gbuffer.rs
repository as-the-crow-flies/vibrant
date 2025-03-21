use std::any::type_name;

use wgpu::{
    CommandEncoder, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::tractogram::Tractogram,
    gpu::Gpu,
    surface::{color::Color, gbuffer::GBuffer, SurfaceBuffer},
};

pub struct TractogramGBufferShading {
    pipeline: RenderPipeline,
}

impl TractogramGBufferShading {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let shading_module = gpu.shader(include_str!("gbuffer.wgsl"));

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(
                        &gpu.device()
                            .create_pipeline_layout(&PipelineLayoutDescriptor {
                                label,
                                bind_group_layouts: &[
                                    &GBuffer::layout(gpu),
                                    &Tractogram::layout(gpu),
                                ],
                                push_constant_ranges: &[],
                            }),
                    ),
                    vertex: VertexState {
                        module: &shading_module,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &shading_module,
                        entry_point: Some("fragment"),
                        targets: &[Some(Color::target_srgb())],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &SurfaceBuffer, tractogram: &Tractogram) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(frame.color().attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.gbuffer().binding(), &[]);
        pass.set_bind_group(1, tractogram.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
