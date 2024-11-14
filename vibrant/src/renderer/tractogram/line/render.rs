use std::any::type_name;

use wgpu::{
    CommandEncoder, FragmentState, MultisampleState, PipelineCompilationOptions, PrimitiveState,
    PrimitiveTopology, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use crate::{
    asset::tractogram::Tractogram,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{depth::Depth, gbuffer::GBuffer, Frame},
};

pub struct TractogramLineRenderRenderer {
    pipeline: RenderPipeline,
}

impl TractogramLineRenderRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(&(Environment::wgsl() + include_str!("render.wgsl")), None);

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    vertex: VertexState {
                        module: &module,
                        entry_point: Some("vertex"),
                        buffers: &[VertexBufferLayout {
                            array_stride: 12,
                            step_mode: VertexStepMode::Vertex,
                            attributes: &[VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            }],
                        }],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: Some("fragment"),
                        targets: &GBuffer::targets(),
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::LineStrip,
                        ..Default::default()
                    },
                    layout: Some(
                        &gpu.pipeline_layout(&[
                            &Tractogram::layout(gpu),
                            &Environment::layout(gpu),
                        ]),
                    ),
                    depth_stencil: Some(Depth::state()),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &frame.gbuffer().attachments(),
            depth_stencil_attachment: Some(frame.depth().attachment()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_vertex_buffer(0, tractogram.vertices().slice(..));
        pass.draw(0..tractogram.vertex_count(), 0..1);
    }
}
