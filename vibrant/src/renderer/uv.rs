use std::any::type_name;

use wgpu::{
    include_wgsl, Color, ColorTargetState, ColorWrites, CommandEncoder, CommandEncoderDescriptor,
    FragmentState, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, StoreOp, TextureView, VertexState,
};

use crate::{
    gpu::Gpu,
    surface::{Frame, FrameView, Surface},
};

pub struct UvRenderer {
    pipeline: RenderPipeline,
}

impl UvRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let module = gpu
            .device()
            .create_shader_module(include_wgsl!("../wgsl/uv.wgsl"));

        let target = ColorTargetState {
            format: Surface::COLOR_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        };

        let pipeline = gpu
            .device()
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(type_name::<Self>()),
                vertex: VertexState {
                    module: &module,
                    entry_point: "vertex",
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &module,
                    entry_point: "fragment",
                    targets: &[Some(target)],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                layout: None,
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        Self { pipeline }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &FrameView) {
        let attachment = RenderPassColorAttachment {
            view: frame.color(),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color {
                    r: 1.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                }),
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

        pass.set_pipeline(&self.pipeline);
        pass.draw(0..4, 0..1);
    }
}
