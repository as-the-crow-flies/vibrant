use std::any::type_name;

use wgpu::{
    FragmentState, MultisampleState, PipelineCompilationOptions, PrimitiveState,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, VertexState,
};

use crate::{
    asset::Tractogram,
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{depth::Depth, gbuffer::GBuffer},
};

pub struct TractogramTubeImpostorRenderer {
    segment: RenderPipeline,

    #[allow(unused)]
    cap: RenderPipeline,
}

impl TractogramTubeImpostorRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let module = gpu.shader(&(Environment::wgsl() + include_str!("impostor.wgsl")), None);

        Self {
            segment: render_pipeline(gpu, &module, "segment"),
            cap: render_pipeline(gpu, &module, "cap"),
        }
    }

    pub(crate) fn render(
        &self,
        cmd: &mut wgpu::CommandEncoder,
        env: &Environment,
        frame: &crate::surface::Frame,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &frame.gbuffer.attachments(),
            depth_stencil_attachment: Some(frame.depth.attachment()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, tractogram.binding_full(), &[]);
        pass.set_bind_group(1, env.binding(), &[]);

        pass.set_pipeline(&self.segment);
        pass.draw(0..4, 0..tractogram.vertex_count() - 2);

        // pass.set_pipeline(&self.cap);
        // pass.set_vertex_buffer(0, tractogram.caps().slice(..));
        // pass.draw(0..4, 0..tractogram.cap_count() - 2);
    }
}

fn render_pipeline(gpu: &Gpu, module: &ShaderModule, entry_point: &str) -> RenderPipeline {
    gpu.device()
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(type_name::<TractogramTubeImpostorRenderer>()),
            layout: Some(
                &gpu.pipeline_layout(&[&Tractogram::layout_full(gpu), &Environment::layout(gpu)]),
            ),
            vertex: VertexState {
                module: &module,
                entry_point: Some(&format!("{}_vertex", entry_point)),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &module,
                entry_point: Some(&format!("{}_fragment", entry_point)),
                targets: &GBuffer::targets(),
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: Some(Depth::state()),
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        })
}
