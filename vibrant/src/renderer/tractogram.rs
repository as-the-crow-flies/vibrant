use std::{any::type_name, cell::RefCell, rc::Rc};

use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState,
    DepthStencilState, FragmentState, IndexFormat, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, StencilFaceState, StencilState, StoreOp, VertexState,
};

use crate::{
    buffer::tractogram::Tractogram,
    gpu::Gpu,
    surface::{FrameView, Surface},
};

use super::camera::Camera;

pub struct TractogramRenderer {
    tractogram: Rc<RefCell<Tractogram>>,
    pipeline: RenderPipeline,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, camera: &Camera, tractogram: Rc<RefCell<Tractogram>>) -> Self {
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
            format: Surface::COLOR_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        };

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    vertex: VertexState {
                        module: &module,
                        entry_point: "vertex",
                        buffers: &[Tractogram::vertex_buffer_layout()],
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
            tractogram,
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

        let tractogram = self.tractogram.borrow();

        if tractogram.count() == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &camera.binding, &[]);
        pass.set_index_buffer(tractogram.indices().slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, tractogram.vertices().slice(..));
        pass.draw_indexed(0..tractogram.count(), 0, 0..1);
    }
}
