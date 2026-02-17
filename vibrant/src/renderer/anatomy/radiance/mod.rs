use std::ops::{Add, Div};

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{radiance::RadianceVolume, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct AnatomyRadiancePipeline {
    cascade: ComputePipeline,
}

impl AnatomyRadiancePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = &gpu.pipeline_layout(&[
            &PhysicalVolume::layout_read(gpu),
            &Environment::layout(gpu),
            &RadianceVolume::layout_read(gpu),
            &RadianceVolume::layout_write(gpu),
        ]);

        Self {
            cascade: gpu.compute(
                "RadiancePipeline",
                layout,
                &gpu.shader(include_str!("cascade.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &mut Environment,
        volume: &PhysicalVolume,
        radiance: &RadianceVolume,
    ) {
        let size = volume.size().add(1).div(2);

        let n_cascades = size.min_element().ilog2();

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.cascade);
        pass.set_bind_group(0, volume.binding_read(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);

        for cascade in (0..n_cascades).rev() {
            pass.set_bind_group(2, radiance.binding_ping_read(), &[]);
            pass.set_bind_group(3, radiance.binding_pong_write(), &[]);
        }
    }
}
