use std::ops::{Add, Div};

use wgpu::*;

use crate::{
    asset::volume::PhysicalVolume,
    gpu::{Gpu, Lazy},
};

pub struct GradientPipeline {
    smooth_x: Lazy<ComputePipeline>,
    smooth_y: Lazy<ComputePipeline>,
    smooth_z: Lazy<ComputePipeline>,
    gradient: Lazy<ComputePipeline>,
}

impl GradientPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            smooth_x: gpu.lazy("Smooth_X", move |gpu| {
                gpu.compute(
                    "Smooth_X",
                    &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_ping(gpu)]),
                    &gpu.shader("renderer/gradient/smooth_x.wgsl"),
                )
            }),
            smooth_y: gpu.lazy("Smooth_Y", move |gpu| {
                gpu.compute(
                    "Smooth_Y",
                    &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_pong(gpu)]),
                    &gpu.shader("renderer/gradient/smooth_y.wgsl"),
                )
            }),
            smooth_z: gpu.lazy("Smooth_Z", move |gpu| {
                gpu.compute(
                    "Smooth_Z",
                    &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_ping(gpu)]),
                    &gpu.shader("renderer/gradient/smooth_z.wgsl"),
                )
            }),
            gradient: gpu.lazy("Gradient", move |gpu| {
                gpu.compute(
                    "Gradient",
                    &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_pong(gpu)]),
                    &gpu.shader("renderer/gradient/gradient.wgsl"),
                )
            }),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, volume: &PhysicalVolume) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        let n_workgroups = volume.size().add(3).div(4);

        pass.set_pipeline(&self.smooth_x);
        pass.set_bind_group(0, volume.binding_gradient_ping(), &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.smooth_y);
        pass.set_bind_group(0, volume.binding_gradient_pong(), &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.smooth_z);
        pass.set_bind_group(0, volume.binding_gradient_ping(), &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.gradient);
        pass.set_bind_group(0, volume.binding_gradient_pong(), &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
    }
}
