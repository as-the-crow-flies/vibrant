use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, CommandEncoder, FragmentState,
    MultisampleState, PrimitiveState, PrimitiveTopology, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, SamplerDescriptor, ShaderStages, TextureSampleType,
    TextureViewDimension, VertexState,
};

use crate::{
    controller::settings::{DebugSetting, Settings},
    gpu::Gpu,
    surface::{buffer::FrameBuffer, Frame},
};

use super::environment::Environment;

pub struct DebugRenderer {
    pipeline: RenderPipeline,
    sampler: BindGroup,
}

impl DebugRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());
        let module = gpu.shader(&(Environment::wgsl() + include_str!("debug.wgsl")), None);

        let sampler_layout = gpu
            .device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }],
            });

        let sampler_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &sampler_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(
                    &gpu.device().create_sampler(&SamplerDescriptor::default()),
                ),
            }],
        });

        Self {
            pipeline: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(
                        &gpu.pipeline_layout(&[
                            &gpu.device()
                                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                                    label,
                                    entries: &[BindGroupLayoutEntry {
                                        binding: 0,
                                        visibility: ShaderStages::all(),
                                        ty: BindingType::Texture {
                                            sample_type: TextureSampleType::Float {
                                                filterable: true,
                                            },
                                            view_dimension: TextureViewDimension::D2,
                                            multisampled: false,
                                        },
                                        count: None,
                                    }],
                                }),
                            &sampler_layout,
                            &Environment::layout(gpu),
                        ]),
                    ),
                    vertex: VertexState {
                        module: &module,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    primitive: PrimitiveState {
                        topology: PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: Some("fragment"),
                        targets: &[Some(FrameBuffer::target_srgb())],
                        compilation_options: Default::default(),
                    }),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            sampler: sampler_binding,
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        settings: &Settings,
    ) {
        if settings.debug == DebugSetting::Disabled {
            return;
        }

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(frame.buffer.attachment_srgb())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame.hierarchy.binding(), &[]);
        pass.set_bind_group(1, &self.sampler, &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
