pub mod cull;

use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites,
    CommandEncoder, FragmentState, MultisampleState, PipelineCompilationOptions, PrimitiveState,
    PrimitiveTopology, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::line::LineSet,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        line::render::raster::transparent::cull::LineTransparentRasterizationCullPipeline,
    },
    surface::{color::ColorBuffer, kbuffer::KBuffer, Frame},
};

pub struct LineTransparentRasterizationPipeline {
    cull: LineTransparentRasterizationCullPipeline,
    gather: RenderPipeline,
    resolve: RenderPipeline,
}

impl LineTransparentRasterizationPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("../common.wgsl");
        let gather_module = &gpu.shader(&(common.to_string() + include_str!("gather.wgsl")));

        let target = ColorTargetState {
            format: ColorBuffer::FORMAT_SRGB,
            blend: Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::OneMinusDstAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::OneMinusDstAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            write_mask: ColorWrites::all(),
        };

        Self {
            cull: LineTransparentRasterizationCullPipeline::new(gpu),
            gather: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("Rasterization::Render"),
                    layout: Some(&gpu.pipeline_layout(&[
                        &LineSet::layout(gpu, true),
                        &KBuffer::layout(gpu),
                        &Frame::layout(gpu),
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
                        targets: &[Some(target.clone())],
                    }),
                    depth_stencil: None,
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
                &gpu.pipeline_layout(&[&KBuffer::layout(gpu), &Environment::layout(gpu)]),
                target,
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
        settings: &Settings,
    ) {
        frame.kbuffer().clear(cmd);
        frame.opacity().clear(cmd);

        let slice_size = line.len().div_ceil(settings.slice_count);

        for slice in 0..settings.slice_count {
            let start = (slice * slice_size).min(line.len());
            let end = ((slice + 1) * slice_size).min(line.len());

            if settings.culling && slice != 0 {
                self.cull
                    .dispatch(cmd, frame, environment, line, start, end);
            }

            self.render_slice(cmd, frame, environment, line, start, end);
        }

        self.resolve(cmd, frame, environment);
    }

    fn render_slice(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
        start: u32,
        end: u32,
    ) {
        let attachment = if start == 0 {
            frame.color().attachment_srgb_clear()
        } else {
            frame.color().attachment_srgb()
        };

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(attachment)],
            label: Some("Rasterization"),
            ..Default::default()
        });

        pass.set_pipeline(&self.gather);
        pass.set_bind_group(0, line.sorted().binding(true), &[]);
        pass.set_bind_group(1, frame.kbuffer().binding(), &[]);
        pass.set_bind_group(2, frame.binding(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.draw(0..6, start..end);
    }

    fn resolve(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment_srgb())],
            label: Some("Rasterization"),
            ..Default::default()
        });

        pass.set_pipeline(&self.resolve);
        pass.set_bind_group(0, frame.kbuffer().binding(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
