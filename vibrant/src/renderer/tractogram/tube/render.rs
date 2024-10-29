use std::{any::type_name, f32};

use wgpu::{
    FragmentState, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PrimitiveState, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, StoreOp, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use crate::{asset::Tractogram, gpu::Gpu, renderer::environment::Environment, surface::Surface};

pub struct TractogramTubeRenderRenderer {
    pipeline: RenderPipeline,
}

impl TractogramTubeRenderRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(&(Environment::wgsl() + include_str!("render.wgsl")), None);

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(
                        &gpu.pipeline_layout(&[
                            &Tractogram::layout(gpu),
                            &Environment::layout(gpu),
                        ]),
                    ),
                    vertex: VertexState {
                        module: &module,
                        entry_point: "vertex",
                        buffers: &[VertexBufferLayout {
                            array_stride: 12,
                            step_mode: VertexStepMode::Instance,
                            attributes: &[
                                VertexAttribute {
                                    format: VertexFormat::Float32x3,
                                    offset: 0,
                                    shader_location: 0,
                                },
                                VertexAttribute {
                                    format: VertexFormat::Float32x3,
                                    offset: 12,
                                    shader_location: 1,
                                },
                                VertexAttribute {
                                    format: VertexFormat::Float32x3,
                                    offset: 24,
                                    shader_location: 2,
                                },
                            ],
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: "fragment",
                        targets: &[Some(Surface::color_srgb_target())],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    depth_stencil: Some(Surface::depth_target()),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
        }
    }

    pub(crate) fn render(
        &self,
        cmd: &mut wgpu::CommandEncoder,
        env: &Environment,
        frame: &crate::surface::Frame,
        tractogram: &Tractogram,
    ) {
        let color_attachment = RenderPassColorAttachment {
            view: frame.color_srgb(),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        };

        let depth_stencil_attachment = RenderPassDepthStencilAttachment {
            view: frame.depth(),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        };

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(depth_stencil_attachment),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, env.binding(), &[]);
        pass.set_vertex_buffer(0, tractogram.vertices().slice(..));
        pass.draw(0..4, 0..tractogram.count() - 2);
    }
}
