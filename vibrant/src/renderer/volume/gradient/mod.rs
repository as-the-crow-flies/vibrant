use std::ops::{Add, Div};

use wgpu::*;

use crate::{asset::volume::PhysicalVolume, gpu::Gpu};

pub struct GradientPipeline {
    smooth_x: ComputePipeline,
    smooth_y: ComputePipeline,
    smooth_z: ComputePipeline,
    gradient: ComputePipeline,
}

impl GradientPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("common.wgsl");

        Self {
            smooth_x: gpu.compute(
                "Smooth_X",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_ping(gpu)]),
                &gpu.shader(&[common, include_str!("smooth_x.wgsl")].concat()),
            ),
            smooth_y: gpu.compute(
                "Smooth_Y",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_pong(gpu)]),
                &gpu.shader(&[common, include_str!("smooth_y.wgsl")].concat()),
            ),
            smooth_z: gpu.compute(
                "Smooth_Z",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_ping(gpu)]),
                &gpu.shader(&[common, include_str!("smooth_z.wgsl")].concat()),
            ),
            gradient: gpu.compute(
                "Gradient",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient_pong(gpu)]),
                &gpu.shader(&[common, include_str!("gradient.wgsl")].concat()),
            ),
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
