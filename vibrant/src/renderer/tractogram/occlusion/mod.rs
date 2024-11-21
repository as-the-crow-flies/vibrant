use itertools::Itertools;
use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{occlusion::Occlusion, scalar::ScalarTexture, Density, Tractogram, TractogramCount},
    gpu::Gpu,
    renderer::{constants::Constants, environment::Environment, services::push::Push},
};

pub struct TractogramOcclusionComputeRenderer {
    constants: Constants,
    push: Push,
    compute: ComputePipeline,
    filter: ComputePipeline,
}

impl TractogramOcclusionComputeRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let n_powers_of_two = constants.volume_xyz.ilog2() + 1;
        let powers_of_two = (0..=n_powers_of_two)
            .map(|i| 2u32.pow(i) as f32 / constants.volume_xyz as f32)
            .collect_vec();

        Self {
            constants: constants.clone(),
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
            filter: gpu.compute(
                &gpu.pipeline_layout(&[
                    &ScalarTexture::layout(gpu),
                    &Tractogram::layout_full_write(gpu),
                    &Environment::layout(gpu),
                    &TractogramCount::layout(gpu),
                ]),
                &gpu.shader(
                    &(Environment::wgsl() + include_str!("filter.wgsl")),
                    Some(&constants),
                ),
                "compute",
            ),
            push: Push::new(gpu, &powers_of_two),
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
        self.trace(cmd, environment, density, occlusion);
        self.filter(cmd, environment, occlusion, tractogram);
    }

    fn trace(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        density: &Density,
        occlusion: &Occlusion,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());
        let size = self.constants.num_workgroups_volume();

        pass.set_pipeline(&self.compute);

        let even_length = self.push.len() % 2 == 0;

        for index in 0..self.push.len() {
            let even_index = index % 2 == 0;

            let src = if index == 0 {
                density.texture().binding()
            } else if even_length == even_index {
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

    fn filter(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        occlusion: &Occlusion,
        tractogram: &Tractogram,
    ) {
        tractogram.count().clear(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        let count = tractogram
            .vertex_count()
            .div_ceil(self.constants.workgroup_x)
            .div_ceil(2); // Two segments are handles per thread

        pass.set_pipeline(&self.filter);
        pass.set_bind_group(0, occlusion.binding(), &[]);
        pass.set_bind_group(1, tractogram.binding_full_write(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, tractogram.count().binding(), &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
}
