use std::any::type_name;

use wgpu::{
    Face, FragmentState, MultisampleState, PipelineCompilationOptions, PrimitiveState,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{asset::Tractogram, gpu::Gpu, renderer::environment::Environment, surface::Surface};

pub struct TractogramTubeRaycastRenderer {
    pipeline: RenderPipeline,
}

impl TractogramTubeRaycastRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let module = gpu.shader(&(Environment::wgsl() + include_str!("raycast.wgsl")), None);

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some(type_name::<TractogramTubeRaycastRenderer>()),
                    layout: Some(&gpu.pipeline_layout(&[
                        &Tractogram::layout_full(gpu),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: &module,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: Some("fragment"),
                        targets: &[
                            Some(Surface::position_target()),
                            Some(Surface::normal_target()),
                            Some(Surface::tangent_target()),
                        ],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        cull_mode: Some(Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(Surface::depth_target()),
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
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
            color_attachments: &frame.gbuffer_attachment(),
            depth_stencil_attachment: Some(frame.depth_attachment()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, tractogram.binding_full(), &[]);
        pass.set_bind_group(1, env.binding(), &[]);

        pass.set_pipeline(&self.pipeline);
        pass.draw(0..14, 0..tractogram.vertex_count() - 2);
    }
}
