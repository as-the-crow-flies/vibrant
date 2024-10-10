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

use super::{constants::Constants, environment::Environment};

pub struct TractogramRenderer {
    geometry: RenderPipeline,
    shading: RenderPipeline,
}

impl TractogramRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let geometry_module = gpu.shader(
            &(Environment::wgsl() + include_str!("wgsl/tractogram_render_geometry.wgsl")),
            Some(constants),
        );

        let shading_module = gpu.shader(
            &(Environment::wgsl() + include_str!("wgsl/tractogram_render_shading.wgsl")),
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
                        targets: &[
                            Some(Surface::position_target()),
                            Some(Surface::normal_target()),
                        ],
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
                                    &Environment::layout(gpu),
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
                                    &Surface::gbuffer(gpu),
                                    &Density::layout_render(gpu),
                                    &Environment::layout(gpu),
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
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
    ) {
        self.geometry(cmd, environment, frame, tractogram);
        self.shading(cmd, environment, frame, density);
    }

    fn geometry(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: frame.position(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: frame.normal(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                }),
            ],
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
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_vertex_buffer(0, tractogram.vertices().slice(..));
        pass.draw(0..tractogram.count(), 0..1);
    }

    fn shading(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        density: &Density,
    ) {
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
        pass.set_bind_group(0, frame.gbuffer(), &[]);
        pass.set_bind_group(1, density.binding_render(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
