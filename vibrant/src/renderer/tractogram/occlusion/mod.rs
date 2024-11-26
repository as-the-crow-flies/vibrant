use itertools::Itertools;
use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{filter::Filter, occlusion::Occlusion, scalar::ScalarTexture, Density, Tractogram},
    gpu::Gpu,
    renderer::{constants::Constants, environment::Environment, services::push::Push},
};

pub struct TractogramOcclusionCompute {
    constants: Constants,
    push: Push,
    copy: ComputePipeline,
    compute: ComputePipeline,
    mipmap: ComputePipeline,
    filter: ComputePipeline,
}

impl TractogramOcclusionCompute {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let steps = (0..=constants.volume_xyz.ilog2())
            .map(|i| 2u32.pow(i) as f32 / constants.volume_xyz as f32)
            .collect_vec();

        Self {
            constants: constants.clone(),
            copy: gpu.compute(
                &gpu.pipeline_layout(&[
                    &ScalarTexture::layout(gpu),
                    &ScalarTexture::layout_write(gpu),
                ]),
                &gpu.shader(
                    &(Environment::wgsl() + include_str!("copy.wgsl")),
                    Some(&constants),
                ),
                "compute",
            ),
            compute: gpu.compute(
                &gpu.pipeline_layout(&[
                    &ScalarTexture::layout(gpu),
                    &ScalarTexture::layout_write(gpu),
                    &Environment::layout(gpu),
                    &Push::layout(gpu),
                ]),
                &gpu.shader(
                    &(Environment::wgsl() + include_str!("compute.wgsl")),
                    Some(&constants),
                ),
                "compute",
            ),
            mipmap: gpu.compute(
                &gpu.pipeline_layout(&[&ScalarTexture::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl"), Some(constants)),
                "main",
            ),
            filter: gpu.compute(
                &gpu.pipeline_layout(&[
                    &ScalarTexture::layout(gpu),
                    &Tractogram::layout(gpu),
                    &Environment::layout(gpu),
                    &Filter::layout_write(gpu),
                ]),
                &gpu.shader(
                    &(Environment::wgsl() + include_str!("filter.wgsl")),
                    Some(&constants),
                ),
                "compute",
            ),
            push: Push::new(gpu, &steps),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        density: &Density,
        occlusion: &Occlusion,
        tractogram: &Tractogram,
    ) {
        self.copy(cmd, density, occlusion);
        self.trace(cmd, environment, occlusion);
        self.mipmap(cmd, occlusion);
        self.filter(cmd, environment, occlusion, tractogram);
    }

    pub fn copy(&self, cmd: &mut CommandEncoder, density: &Density, occlusion: &Occlusion) {
        let src = density.texture().binding();
        let dst = if self.push.len() % 2 == 0 {
            occlusion.ping().binding_write()
        } else {
            occlusion.pong().binding_write()
        };

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());
        let size = self.constants.num_workgroups_volume();

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, src, &[]);
        pass.set_bind_group(1, dst, &[]);
        pass.dispatch_workgroups(size, size, size);
    }

    pub fn trace(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        occlusion: &Occlusion,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());
        let size = self.constants.num_workgroups_volume();

        pass.set_pipeline(&self.compute);

        let even_length = self.push.len() % 2 == 0;

        for index in 0..self.push.len() {
            let even_index = index % 2 == 0;

            let src = if even_length == even_index {
                occlusion.ping().binding()
            } else {
                occlusion.pong().binding()
            };

            let dst = if even_length == even_index {
                occlusion.pong().binding_write()
            } else {
                occlusion.ping().binding_write()
            };

            pass.set_bind_group(0, src, &[]);
            pass.set_bind_group(1, dst, &[]);
            pass.set_bind_group(2, environment.binding(), &[]);

            self.push.apply(&mut pass, 3, index);

            pass.dispatch_workgroups(size, size, size);
        }
    }

    pub fn mipmap(&self, cmd: &mut CommandEncoder, occlusion: &Occlusion) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());
        let mut size = self.constants.num_workgroups_volume();

        pass.set_pipeline(&self.mipmap);

        for binding in occlusion.bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(size, size, size);

            size /= 2;
        }
    }

    pub fn filter(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        occlusion: &Occlusion,
        tractogram: &Tractogram,
    ) {
        tractogram.filter_culling().clear(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        let count = tractogram
            .vertex_count()
            .div_ceil(self.constants.workgroup_x)
            .div_ceil(2); // Two segments are handles per thread

        pass.set_pipeline(&self.filter);
        pass.set_bind_group(0, occlusion.binding(), &[]);
        pass.set_bind_group(1, tractogram.binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, tractogram.filter_culling().binding_write(), &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
}
