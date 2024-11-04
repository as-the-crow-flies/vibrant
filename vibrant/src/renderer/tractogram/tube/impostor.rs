use std::any::type_name;

use wgpu::{
    FragmentState, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PrimitiveState, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, StoreOp,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use crate::{asset::Tractogram, gpu::Gpu, renderer::environment::Environment, surface::Surface};

pub struct TractogramTubeImpostorRenderer {
    segment: RenderPipeline,
    cap: RenderPipeline,
}

impl TractogramTubeImpostorRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let module = gpu.shader(&(Environment::wgsl() + include_str!("impostor.wgsl")), None);

        Self {
            segment: render_pipeline(
                gpu,
                &module,
                "segment",
                VertexBufferLayout {
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
                },
            ),
            cap: render_pipeline(
                gpu,
                &module,
                "cap",
                VertexBufferLayout {
                    array_stride: 24,
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
                    ],
                },
            ),
        }
    }

    pub(crate) fn render(
        &self,
        cmd: &mut wgpu::CommandEncoder,
        env: &Environment,
        frame: &crate::surface::Frame,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &frame.gbuffer_attachment(),
            depth_stencil_attachment: Some(frame.depth_attachment()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, env.binding(), &[]);

        pass.set_pipeline(&self.segment);
        pass.set_vertex_buffer(0, tractogram.vertices().slice(..));
        pass.draw(0..4, 0..tractogram.vertex_count() - 2);

        // pass.set_pipeline(&self.cap);
        // pass.set_vertex_buffer(0, tractogram.caps().slice(..));
        // pass.draw(0..4, 0..tractogram.cap_count() - 2);
    }
}

fn render_pipeline(
    gpu: &Gpu,
    module: &ShaderModule,
    entry_point: &str,
    layout: VertexBufferLayout,
) -> RenderPipeline {
    gpu.device()
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(type_name::<TractogramTubeImpostorRenderer>()),
            layout: Some(
                &gpu.pipeline_layout(&[&Tractogram::layout(gpu), &Environment::layout(gpu)]),
            ),
            vertex: VertexState {
                module: &module,
                entry_point: &format!("{}_vertex", entry_point),
                buffers: &[layout],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &module,
                entry_point: &format!("{}_fragment", entry_point),
                targets: &[
                    Some(Surface::position_target()),
                    Some(Surface::normal_target()),
                    Some(Surface::tangent_target()),
                ],
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
        })
}
