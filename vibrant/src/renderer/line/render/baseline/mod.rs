use wgpu::{
    ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState,
    DepthStencilState, FragmentState, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, StencilState, StoreOp, VertexState,
};

use crate::{
    asset::line::LineSet,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, visibility::VisibilityBuffer, Frame},
};

pub struct LineBaselineRasterizationPipeline {
    gather: RenderPipeline,
    resolve: RenderPipeline,
}

impl LineBaselineRasterizationPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("../rasterization/common.wgsl");
        let gather_module = &gpu.shader(&(common.to_string() + include_str!("gather.wgsl")));

        Self {
            gather: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("Rasterization::Render"),
                    layout: Some(&gpu.pipeline_layout(&[
                        &LineSet::layout(gpu, true),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: gather_module,
                        entry_point: None,
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    fragment: Some(FragmentState {
                        module: gather_module,
                        entry_point: None,
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: VisibilityBuffer::INDEX_FORMAT,
                            blend: None,
                            write_mask: ColorWrites::all(),
                        })],
                    }),
                    depth_stencil: Some(DepthStencilState {
                        format: VisibilityBuffer::DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Less,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            resolve: gpu.quad(
                "Rasterization::Resolve",
                &gpu.pipeline_layout(&[
                    &LineSet::layout(gpu, true),
                    &VisibilityBuffer::layout(gpu),
                    &Frame::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                ColorBuffer::target_srgb(),
                &gpu.shader(&(common.to_string() + include_str!("resolve.wgsl"))),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
    ) {
        self.gather(cmd, frame, environment, line);
        self.resolve(cmd, frame, environment, line);
    }

    fn gather(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some("Rasterization"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame.visibility().index_view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: frame.visibility().depth_view_base(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        pass.set_pipeline(&self.gather);
        pass.set_bind_group(0, line.binding(true), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..6, 0..line.len());
    }

    fn resolve(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb_clear())],
            label: Some("Rasterization"),
            ..Default::default()
        });

        pass.set_pipeline(&self.resolve);
        pass.set_bind_group(0, line.binding(true), &[]);
        pass.set_bind_group(1, frame.visibility().binding(), &[]);
        pass.set_bind_group(2, frame.binding(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
