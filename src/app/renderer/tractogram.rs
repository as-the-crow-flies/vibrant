use std::any::type_name;

use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction,
    DepthBiasState, DepthStencilState, FragmentState, IndexFormat, LoadOp, MultisampleState,
    Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, StencilFaceState, StencilState,
    StoreOp, TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

use crate::app::{
    gpu::Gpu,
    loader::Tractogram,
    surface::{Frame, FrameView, Surface},
};

use super::camera::Camera;

pub struct TractogramRenderer {
    pipeline: RenderPipeline,
    indices: Buffer,
    vertices: Buffer,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, camera: &Camera, tractogram: &Tractogram) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu
            .device()
            .create_shader_module(include_wgsl!("../wgsl/tractogram.wgsl"));

        let pipeline_layout = gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[&camera.layout],
                push_constant_ranges: &[],
            });

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
                    label,
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
                    layout: Some(&pipeline_layout),
                    depth_stencil: Some(DepthStencilState {
                        format: Surface::DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Less,
                        stencil: StencilState {
                            front: StencilFaceState::IGNORE,
                            back: StencilFaceState::IGNORE,
                            read_mask: 0,
                            write_mask: 0,
                        },
                        bias: DepthBiasState {
                            constant: 0,
                            slope_scale: 0.0,
                            clamp: 0.0,
                        },
                    }),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            indices: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(&tractogram.indices),
                usage: BufferUsages::INDEX,
            }),
            vertices: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(&tractogram.vertices),
                usage: BufferUsages::VERTEX,
            }),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, camera: &Camera, frame: &FrameView) {
        let color_attachment = RenderPassColorAttachment {
            view: frame.color(),
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
                store: StoreOp::Discard,
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

        let count = (self.indices.size() / 4) as u32;

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &camera.binding, &[]);
        pass.set_index_buffer(self.indices.slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw_indexed(0..count, 0, 0..1);
    }
}
