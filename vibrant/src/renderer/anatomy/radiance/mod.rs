use std::iter::zip;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{radiance::RadianceVolume, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct AnatomyRadiancePipeline {
    cascade: ComputePipeline,
    collect: ComputePipeline,
}

impl AnatomyRadiancePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("common.wgsl");

        Self {
            cascade: gpu.compute(
                "RadiancePipeline",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &RadianceVolume::layout_cascade(gpu),
                ]),
                &gpu.shader(&[common, include_str!("cascade.wgsl")].concat()),
            ),
            collect: gpu.compute(
                "RadiancePipeline",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &RadianceVolume::layout_write(gpu),
                ]),
                &gpu.shader(&[common, include_str!("collect.wgsl")].concat()),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        volume: &PhysicalVolume,
        radiance: &RadianceVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.cascade);

        pass.set_bind_group(0, volume.binding_read(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);

        for (resolution, binding) in
            zip(radiance.cascade_resolutions(), radiance.binding_cascades()).rev()
        {
            pass.set_bind_group(2, binding, &[]);
            pass.dispatch_workgroups(
                resolution.x.div_ceil(4),
                resolution.y.div_ceil(4),
                resolution.z.div_ceil(4),
            );
        }

        pass.set_pipeline(&self.collect);
        pass.set_bind_group(2, radiance.binding_write(), &[]);
        pass.dispatch_workgroups(
            radiance.radiance_resolution().x.div_ceil(4),
            radiance.radiance_resolution().y.div_ceil(4),
            radiance.radiance_resolution().z.div_ceil(4),
        );
    }
}
