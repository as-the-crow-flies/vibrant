use std::any::type_name;

use wgpu::{
    Color, CommandEncoder, FragmentState, IndexFormat, LoadOp, MultisampleState, Operations,
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
    geometry: RenderPipeline,
    shading: RenderPipeline,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let geometry_module = gpu.shader(
            include_str!("wgsl/tractogram_render_geometry.wgsl"),
            Some(constants),
        );

        let shading_module = gpu.shader(
            include_str!("wgsl/tractogram_render_shading.wgsl"),
            Some(constants),
        );

        Self {
            geometry: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    vertex: VertexState {
                        module: &geometry_module,
                        entry_point: "vertex",
                        buffers: &[Tractogram::vertex_buffer_layout()],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &geometry_module,
                        entry_point: "fragment",
                        targets: &[Some(Surface::gbuffer_target())],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::LineStrip,
                        strip_index_format: Some(IndexFormat::Uint32),
                        ..Default::default()
                    },
                    layout: Some(
                        &gpu.device()
                            .create_pipeline_layout(&PipelineLayoutDescriptor {
                                label,
                                bind_group_layouts: &[
                                    &Tractogram::layout(gpu),
                                    &Camera::layout(gpu),
                                ],
                                push_constant_ranges: &[],
                            }),
                    ),
                    depth_stencil: Some(Surface::depth_target()),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            shading: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(
                        &gpu.device()
                            .create_pipeline_layout(&PipelineLayoutDescriptor {
                                label,
                                bind_group_layouts: &[
                                    &Surface::gbuffer_layout(gpu),
                                    &Density::layout_render(gpu),
                                    &Camera::layout(gpu),
                                ],
                                push_constant_ranges: &[],
                            }),
                    ),
                    vertex: VertexState {
                        module: &shading_module,
                        entry_point: "vertex",
                        buffers: &[],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &shading_module,
                        entry_point: "fragment",
                        targets: &[Some(Surface::color_srgb_target())],
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
        self.geometry(cmd, camera, frame, tractogram, count);
        self.shading(cmd, camera, frame, density);
    }

    fn geometry(
        &self,
        cmd: &mut CommandEncoder,
        camera: &Camera,
        frame: &Frame,
        tractogram: &Tractogram,
        count: u32,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame.gbuffer(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: frame.depth(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.geometry);
        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, camera.binding(), &[]);
        pass.set_index_buffer(tractogram.indices().slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, tractogram.vertices().slice(..));
        pass.draw_indexed(0..count, 0, 0..1);
    }

    fn shading(&self, cmd: &mut CommandEncoder, camera: &Camera, frame: &Frame, density: &Density) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame.color_srgb(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.shading);
        pass.set_bind_group(0, frame.gbuffer_binding(), &[]);
        pass.set_bind_group(1, density.binding_render(), &[]);
        pass.set_bind_group(2, camera.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
