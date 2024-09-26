use std::{any::type_name, fs};

use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, FragmentState,
    IndexFormat, LoadOp, MultisampleState, Operations, PipelineCompilationOptions, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, StoreOp, TextureView, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode,
};

use crate::app::{gpu::Gpu, loader::load_tck, surface::Surface};

pub struct TractogramRenderer {
    pipeline: RenderPipeline,
    indices: Buffer,
    vertices: Buffer,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let tractogram = load_tck(&fs::read("../../assets/CC.tck").unwrap());

        let module = gpu
            .device()
            .create_shader_module(include_wgsl!("../wgsl/tractogram.wgsl"));

        let target = ColorTargetState {
            format: Surface::VIEW_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        };

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: 12,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some(type_name::<Self>()),
                    vertex: VertexState {
                        module: &module,
                        entry_point: "vertex",
                        buffers: &[vertex_buffer_layout],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: "fragment",
                        targets: &[Some(target)],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::LineStrip,
                        strip_index_format: Some(IndexFormat::Uint32),
                        ..Default::default()
                    },
                    layout: None,
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            indices: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label: Some(type_name::<Self>()),
                contents: bytemuck::cast_slice(&tractogram.indices),
                usage: BufferUsages::INDEX,
            }),
            vertices: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label: Some(type_name::<Self>()),
                contents: bytemuck::cast_slice(&tractogram.vertices),
                usage: BufferUsages::VERTEX,
            }),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, view: &TextureView) {
        let attachment = RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        };

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let count = (self.indices.size() / 4) as u32;

        pass.set_pipeline(&self.pipeline);
        pass.set_index_buffer(self.indices.slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw_indexed(0..count, 0, 0..1);
    }
}
