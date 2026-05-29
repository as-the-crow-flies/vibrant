use std::iter::zip;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{hdri::HdriBuffer, radiance::RadianceVolume, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct ExplicitCascadePipeline {
    cascade: ComputePipeline,
}

impl ExplicitCascadePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            cascade: gpu.compute(
                "RadiancePipeline",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &RadianceVolume::layout_cascade(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                &gpu.shader(&include_str!("cascade.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        volume: &PhysicalVolume,
        radiance: &RadianceVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.cascade);

        pass.set_bind_group(0, volume.binding_read(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        for (cascade, binding) in zip(radiance.cascades(), radiance.binding_cascades()).rev() {
            pass.set_bind_group(2, binding, &[]);
            pass.dispatch_workgroups(
                cascade.size().x.div_ceil(4),
                cascade.size().y.div_ceil(4),
                cascade.size().z.div_ceil(4),
            );
        }
    }
}
