use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Face, FragmentState, MultisampleState, PipelineCompilationOptions,
    PrimitiveState, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::{filter::Filter, Tractogram},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{depth::Depth, gbuffer::GBuffer, SurfaceBuffer},
};

pub struct TractogramTubeGeometry {
    pipeline: RenderPipeline,
    indirect: Buffer,
}

impl TractogramTubeGeometry {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<TractogramTubeGeometry>());
        let module = gpu.shader(&(Environment::wgsl() + include_str!("tube.wgsl")), None);

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[
                        &Tractogram::layout(gpu),
                        &Filter::layout_read(gpu),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: &module,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: Some("fragment"),
                        targets: &GBuffer::targets(),
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        cull_mode: Some(Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(Depth::state()),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            indirect: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytes_of(&[14u32, 0, 0, 0]),
                usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut wgpu::CommandEncoder,
        env: &Environment,
        frame: &SurfaceBuffer,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        cmd.copy_buffer_to_buffer(filter.count(), 0, &self.indirect, 4, 4);

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &frame.gbuffer().attachments(),
            depth_stencil_attachment: Some(frame.depth().attachment()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, filter.binding_read(), &[]);
        pass.set_bind_group(2, env.binding(), &[]);

        pass.set_pipeline(&self.pipeline);
        pass.draw_indirect(&self.indirect, 0);
    }
}
