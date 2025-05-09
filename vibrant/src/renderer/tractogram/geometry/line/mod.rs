use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, CommandEncoder, FragmentState, MultisampleState,
    PipelineCompilationOptions, PrimitiveState, PrimitiveTopology, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::{filter::Filter, tractogram::Tractogram},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{gbuffer::GBuffer, SurfaceBuffer},
};

pub struct TractogramLineGeometry {
    pipeline: RenderPipeline,
    indirect: Buffer,
}

impl TractogramLineGeometry {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(include_str!("line.wgsl"));

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    vertex: VertexState {
                        module: &module,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: Some("fragment"),
                        targets: &GBuffer::target_tangent(),
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::LineStrip,
                        ..Default::default()
                    },
                    layout: Some(&gpu.pipeline_layout(&[
                        &Tractogram::layout(gpu),
                        &Filter::layout_read(gpu),
                        &Environment::layout(gpu),
                    ])),
                    depth_stencil: Some(GBuffer::depth_state()),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            indirect: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytes_of(&[2u32, 0, 0, 0]),
                usage: BufferUsages::COPY_DST | BufferUsages::INDIRECT,
            }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &SurfaceBuffer,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        cmd.copy_buffer_to_buffer(&filter.count(), 0, &self.indirect, 4, 4);

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &frame.gbuffer().attachment_tangent(),
            depth_stencil_attachment: Some(frame.gbuffer().depth_attachment()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, filter.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.draw_indirect(&self.indirect, 0);
    }
}
