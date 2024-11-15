use std::{any::type_name, iter::zip};

use wgpu::{
    CommandEncoder, FragmentState, PrimitiveState, PrimitiveTopology, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::Density,
    gpu::Gpu,
    renderer::{constants::Constants, environment::Environment},
    surface::{hierarchy::DepthHierarchy, Frame},
};

pub struct TractogramOcclusionRenderer {
    occlusion: RenderPipeline,
    mipmap: RenderPipeline,
}

impl TractogramOcclusionRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let occlusion = gpu.shader(
            &(Environment::wgsl() + include_str!("occlusion.wgsl")),
            Some(constants),
        );

        let mipmap = gpu.shader(include_str!("mipmap.wgsl"), None);

        Self {
            occlusion: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[
                        &Density::layout_render(gpu),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: &occlusion,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    fragment: Some(FragmentState {
                        module: &occlusion,
                        entry_point: Some("fragment"),
                        targets: &[Some(DepthHierarchy::target())],
                        compilation_options: Default::default(),
                    }),
                    multisample: Default::default(),
                    depth_stencil: None,
                    multiview: None,
                    cache: None,
                }),
            mipmap: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[&DepthHierarchy::layout(gpu)])),
                    vertex: VertexState {
                        module: &mipmap,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    fragment: Some(FragmentState {
                        module: &mipmap,
                        entry_point: Some("fragment"),
                        targets: &[Some(DepthHierarchy::target())],
                        compilation_options: Default::default(),
                    }),
                    multisample: Default::default(),
                    depth_stencil: None,
                    multiview: None,
                    cache: None,
                }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        density: &Density,
    ) {
        self.occlusion(cmd, frame, environment, density);
        self.mipmap(cmd, frame.hierarchy);
    }

    fn occlusion(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        density: &Density,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.hierarchy.attachment())],
            ..Default::default()
        });

        pass.set_pipeline(&self.occlusion);
        pass.set_bind_group(0, density.binding_render(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }

    fn mipmap(&self, cmd: &mut CommandEncoder, hierarchy: &DepthHierarchy) {
        for (source, destination) in zip(hierarchy.bindings(), hierarchy.attachments()) {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(destination)],
                ..Default::default()
            });

            pass.set_pipeline(&self.mipmap);
            pass.set_bind_group(0, source, &[]);
            pass.draw(0..4, 0..1);
        }
    }
}
