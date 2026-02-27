use std::any::type_name;

use wgpu::*;

use crate::{asset::volume::PhysicalVolume, gpu::Gpu};

pub struct GradientPipeline {
    compute: ComputePipeline,
}

impl GradientPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            compute: gpu.compute(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient(gpu)]),
                &gpu.shader(include_str!("sobel.wgsl")),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, volume: &PhysicalVolume) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.compute);
        pass.set_bind_group(0, volume.binding_gradient(), &[]);
        pass.dispatch_workgroups(
            volume.size().x.div_ceil(4),
            volume.size().y.div_ceil(4),
            volume.size().z.div_ceil(4),
        );
    }
}
