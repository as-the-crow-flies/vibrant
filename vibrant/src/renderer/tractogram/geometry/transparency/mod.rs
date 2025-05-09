use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Face,
    FragmentState, MultisampleState, PipelineCompilationOptions, PrimitiveState, PrimitiveTopology,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, StencilState, VertexState,
};

use crate::{
    asset::{
        filter::Filter,
        scalar::{ScalarTexture2D, ScalarTexture3D},
        tractogram::Tractogram,
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::Color, gbuffer::GBuffer, slice::SliceBuffer, SurfaceBuffer},
};

pub struct TractogramTransparentGeometry {
    rasterize: RenderPipeline,
    resolve: RenderPipeline,
    indirect: Buffer,
}

impl TractogramTransparentGeometry {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<TractogramTransparentGeometry>());
        let rasterize = gpu.shader(include_str!("rasterize.wgsl"));
        let resolve = gpu.shader(include_str!("resolve.wgsl"));

        Self {
            rasterize: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[
                        &Tractogram::layout(gpu),
                        &Filter::layout_read(gpu),
                        &ScalarTexture3D::layout(gpu),
                        &ScalarTexture2D::layout(gpu),
                        &SliceBuffer::layout(gpu, false),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: &rasterize,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &rasterize,
                        entry_point: Some("fragment"),
                        targets: &[],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleStrip,
                        cull_mode: Some(Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(DepthStencilState {
                        format: GBuffer::DEPTH_FORMAT,
                        depth_write_enabled: false,
                        depth_compare: CompareFunction::Always,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            resolve: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[
                        &SliceBuffer::layout(gpu, true),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: &resolve,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &resolve,
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
            indirect: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytes_of(&[14u32, 0, 0, 0]),
                usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        env: &Environment,
        frame: &SurfaceBuffer,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        self.rasterize(cmd, env, frame, tractogram, filter);
        self.resolve(cmd, env, frame);
    }

    fn rasterize(
        &self,
        cmd: &mut CommandEncoder,
        env: &Environment,
        frame: &SurfaceBuffer,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        cmd.copy_buffer_to_buffer(filter.count(), 0, &self.indirect, 4, 4);

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            depth_stencil_attachment: Some(frame.gbuffer().depth_attachment()),
            ..Default::default()
        });

        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, filter.binding_read(), &[]);
        pass.set_bind_group(2, frame.density().volume().binding(), &[]);
        pass.set_bind_group(3, frame.slice().hiz().binding(), &[]);
        pass.set_bind_group(4, frame.slice().binding(false), &[]);
        pass.set_bind_group(5, env.binding(), &[]);

        pass.set_pipeline(&self.rasterize);
        pass.draw_indirect(&self.indirect, 0);
    }

    fn resolve(&self, cmd: &mut CommandEncoder, env: &Environment, frame: &SurfaceBuffer) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(frame.color().attachment_srgb())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.resolve);
        pass.set_bind_group(0, frame.slice().binding(true), &[]);
        pass.set_bind_group(1, env.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
