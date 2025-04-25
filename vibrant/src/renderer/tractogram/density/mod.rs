use std::any::type_name;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, CommandEncoder, ComputePassDescriptor, ComputePipeline,
    PipelineLayoutDescriptor,
};

use crate::{
    asset::{filter::Filter, scalar::ScalarTexture3D, tractogram::Tractogram},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::density::Density,
};

pub struct TractogramDensityPipeline {
    voxelize: ComputePipeline,
    copy: ComputePipeline,
    mipmap: ComputePipeline,
    indirect: Buffer,
}

impl TractogramDensityPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        Self {
            voxelize: gpu.compute(
                "Density::Voxelize",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &Density::layout(gpu),
                            &Tractogram::layout(gpu),
                            &Environment::layout(gpu),
                            &Filter::layout_read(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    &(Environment::wgsl()
                        + include_str!("common.wgsl")
                        + include_str!("voxelize.wgsl")),
                ),
                "main",
            ),
            copy: gpu.compute(
                "Density::Copy",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &Density::layout(gpu),
                            &ScalarTexture3D::layout_write(gpu),
                            &Environment::layout(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(
                    &(Environment::wgsl()
                        + include_str!("common.wgsl")
                        + include_str!("copy.wgsl")),
                ),
                "main",
            ),
            mipmap: gpu.compute(
                "Density::MipMap",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[&ScalarTexture3D::layout_mipmap(gpu)],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("mipmap.wgsl")),
                "main",
            ),
            indirect: gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(&[0u32, 1, 1]),
                usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            }),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        tractogram: &Tractogram,
        density: &Density,
    ) {
        density.clear(cmd);

        cmd.copy_buffer_to_buffer(
            tractogram.filter_default().workgroup_count_32(),
            0,
            &self.indirect,
            0,
            4,
        );

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Density"),
            ..Default::default()
        });

        let n = density.texture().width().div_ceil(8);

        pass.set_bind_group(0, density.binding(), &[]);
        pass.set_bind_group(1, tractogram.binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, tractogram.filter_default().binding_read(), &[]);

        pass.set_pipeline(&self.voxelize);
        pass.dispatch_workgroups_indirect(&self.indirect, 0);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(1, density.texture().binding_write(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = density.texture().width().div_ceil(8);

        for binding in density.texture().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap /= 2;
        }
    }
}
