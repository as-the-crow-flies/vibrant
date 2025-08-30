use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ShaderStages,
};

use crate::{
    asset::{
        line::LineSet,
        texture::{MipTexture2D, R32Float},
    },
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{kbuffer::KBuffer, Frame},
};

pub struct LineTransparentRasterizationCullPipeline {
    copy: ComputePipeline,
    mipmap: ComputePipeline,
    cull: ComputePipeline,
    push: Buffer,
    push_binding: BindGroup,
}

impl LineTransparentRasterizationCullPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("../../common.wgsl");

        let copy = gpu.compute(
            "Rasterization::Cull::Copy",
            &gpu.pipeline_layout(&[
                &KBuffer::layout_resolve(gpu),
                &MipTexture2D::<R32Float>::layout_write(gpu),
            ]),
            &gpu.shader(&(common.to_string() + include_str!("copy.wgsl"))),
        );

        let mipmap = gpu.compute(
            "Rasterization::Cull::MipMap",
            &gpu.pipeline_layout(&[&MipTexture2D::<R32Float>::layout_mipmap(gpu)]),
            &gpu.shader(include_str!("mipmap.wgsl")),
        );

        let cull = gpu.compute(
            "Rasterization::Cull::Cull",
            &gpu.pipeline_layout(&[
                &LineSet::layout(gpu, false),
                &MipTexture2D::<R32Float>::layout(gpu),
                &Self::push_layout(gpu),
                &Environment::layout(gpu),
            ]),
            &gpu.shader(&(common.to_string() + include_str!("cull.wgsl"))),
        );

        let push = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("Rasterization::Cull::Push"),
            size: 8,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let push_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some("Rasterization::Cull::Push::Binding"),
            layout: &Self::push_layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: push.as_entire_binding(),
            }],
        });

        Self {
            copy,
            mipmap,
            cull,
            push,
            push_binding,
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
        start: u32,
        end: u32,
    ) {
        line.clear_cull(cmd);

        cmd.copy_buffer_to_buffer(line.sorted().linear(), (start * 4) as u64, &self.push, 0, 4);
        cmd.copy_buffer_to_buffer(line.sorted().linear(), (end * 4) as u64, &self.push, 4, 4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Rasterization::Opacity"),
            ..Default::default()
        });

        let mut width = frame.color().width().div_ceil(2).div_ceil(32);
        let mut height = frame.color().height().div_ceil(2).div_ceil(32);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, frame.kbuffer().binding_resolve(), &[]);
        pass.set_bind_group(1, frame.opacity().opacity().binding_write(), &[]);
        pass.dispatch_workgroups(width, height, 1);

        pass.set_pipeline(&self.mipmap);

        for binding in frame.opacity().opacity().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(width, height, 1);

            width = width.div_ceil(2);
            height = height.div_ceil(2);
        }

        pass.set_pipeline(&self.cull);
        pass.set_bind_group(0, line.sorted().binding(false), &[]);
        pass.set_bind_group(1, frame.opacity().opacity().binding(), &[]);
        pass.set_bind_group(2, &self.push_binding, &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups((end - start).div_ceil(1024), 1, 1);
    }

    fn push_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Rasterization::Cull::Push::Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}
