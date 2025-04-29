use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferUsages,
    ColorTargetState, ColorWrites, Face, FragmentState, MultisampleState,
    PipelineCompilationOptions, PrimitiveState, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::{filter::Filter, scalar::ScalarTexture3D, tractogram::Tractogram},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::Color, SurfaceBuffer},
};

pub struct TractogramTransparentGeometry {
    rasterize: RenderPipeline,
    indirect: Buffer,
}

impl TractogramTransparentGeometry {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<TractogramTransparentGeometry>());
        let rasterize = gpu.shader(&(Environment::wgsl() + include_str!("rasterize.wgsl")));

        Self {
            rasterize: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(&gpu.pipeline_layout(&[
                        &Tractogram::layout(gpu),
                        &Filter::layout_read(gpu),
                        &ScalarTexture3D::layout(gpu),
                        &ScalarTexture3D::layout(gpu),
                        &Environment::layout(gpu),
                    ])),
                    vertex: VertexState {
                        module: &rasterize,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &rasterize,
                        entry_point: Some("fragment"),
                        targets: &[Some(ColorTargetState {
                            format: Color::FORMAT_SRGB,
                            blend: Some(BlendState {
                                color: BlendComponent {
                                    src_factor: BlendFactor::One,
                                    dst_factor: BlendFactor::OneMinusSrc,
                                    operation: BlendOperation::Add,
                                },
                                alpha: BlendComponent {
                                    src_factor: BlendFactor::One,
                                    dst_factor: BlendFactor::OneMinusSrc,
                                    operation: BlendOperation::Add,
                                },
                            }),
                            write_mask: ColorWrites::all(),
                        })],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        cull_mode: Some(Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            indirect: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytes_of(&[14u32, 0, 0, 0]),
                usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut wgpu::CommandEncoder,
        env: &Environment,
        frame: &SurfaceBuffer,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        cmd.copy_buffer_to_buffer(filter.count(), 0, &self.indirect, 4, 4);

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(frame.color().attachment_srgb())],
            ..Default::default()
        });

        pass.set_bind_group(0, tractogram.binding(), &[]);
        pass.set_bind_group(1, filter.binding_read(), &[]);
        pass.set_bind_group(2, frame.density().volume().binding(), &[]);
        pass.set_bind_group(3, frame.occlusion().volume().binding(), &[]);
        pass.set_bind_group(4, env.binding(), &[]);

        pass.set_pipeline(&self.rasterize);
        pass.draw_indirect(&self.indirect, 0);
    }
}
