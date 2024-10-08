use std::{any::type_name, ops::Add};

use wgpu::{
    CommandEncoder, FragmentState, IndexFormat, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, StoreOp, VertexState,
};

use crate::{
    asset::{density::Density, tractogram::Tractogram},
    gpu::Gpu,
    surface::{Frame, Surface},
};

use super::{camera::Camera, constants::Constants};

pub struct TractogramRenderer {
    pipeline: RenderPipeline,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(include_str!("wgsl/tractogram_render.wgsl"), Some(constants));

        let pipeline_layout = gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[
                    &Tractogram::layout(gpu),
                    &Density::layout_render(gpu),
                    &Camera::layout(gpu),
                ],
                push_constant_ranges: &[],
            });

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
                        targets: &[Some(Surface::color_srgb_target())],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::LineStrip,
                        strip_index_format: Some(IndexFormat::Uint32),
                        ..Default::default()
                    },
                    layout: Some(&pipeline_layout),
                    depth_stencil: Some(Surface::depth_target()),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        camera: &Camera,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
        count: u32,
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
        pass.set_bind_group(1, density.binding_render(), &[]);
        pass.set_bind_group(2, camera.binding(), &[]);
        pass.set_index_buffer(tractogram.indices().slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, tractogram.vertices().slice(..));
        pass.draw_indexed(0..count, 0, 0..1);
    }
}
