use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, CommandEncoder, ComputePassDescriptor, ComputePipeline, FragmentState,
    MultisampleState, PipelineCompilationOptions, PrimitiveState, PrimitiveTopology,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::{filter::Filter, Tractogram},
    gpu::Gpu,
    renderer::{constants::Constants, environment::Environment},
    surface::{gbuffer::GBuffer, kbuffer::KBuffer, Frame},
};

pub struct TractogramLineSoftwareGeometry {
    clear: ComputePipeline,
    rasterize: ComputePipeline,
    copy: RenderPipeline,
    indirect: Buffer,
    constants: Constants,
}

impl TractogramLineSoftwareGeometry {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(
            &(Environment::wgsl() + include_str!("copy.wgsl")),
            Some(constants),
        );

        Self {
            clear: gpu.compute(
                &gpu.pipeline_layout(&[&KBuffer::layout_write(gpu)]),
                &gpu.shader(include_str!("clear.wgsl"), Some(constants)),
                "compute",
            ),
            rasterize: gpu.compute(
                &gpu.pipeline_layout(&[
                    &KBuffer::layout_write(gpu),
                    &Tractogram::layout(gpu),
                    &Filter::layout_read(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(
                    &(Environment::wgsl() + include_str!("rasterize.wgsl")),
                    Some(constants),
                ),
                "main",
            ),
            copy: gpu
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label,
                    layout: Some(
                        &gpu.pipeline_layout(&[
                            &KBuffer::layout_read(gpu),
                            &Environment::layout(gpu),
                        ]),
                    ),
                    vertex: VertexState {
                        module: &module,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: Some("fragment"),
                        targets: &GBuffer::targets(),
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
            indirect: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytes_of(&[0u32, 1, 1]),
                usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            }),
            constants: constants.clone(),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        self.rasterize(cmd, frame, environment, tractogram, filter);
        self.copy(cmd, frame, environment);
    }

    fn rasterize(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        tractogram: &Tractogram,
        filter: &Filter,
    ) {
        let (x, y) = self.constants.num_workgroups_surface();

        cmd.copy_buffer_to_buffer(filter.workgroup_count(), 0, &self.indirect, 0, 4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, frame.kbuffer.binding_write(), &[]);
        pass.set_bind_group(1, tractogram.binding(), &[]);
        pass.set_bind_group(2, filter.binding_read(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);

        pass.set_pipeline(&self.clear);
        pass.dispatch_workgroups(x, y, 1);

        pass.set_pipeline(&self.rasterize);
        pass.dispatch_workgroups_indirect(&self.indirect, 0);
    }

    fn copy(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &frame.gbuffer.attachments(),
            ..Default::default()
        });

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, frame.kbuffer.binding_read(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
