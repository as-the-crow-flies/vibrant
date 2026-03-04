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
        let layout = &gpu.pipeline_layout(&[&PhysicalVolume::layout_gradient(gpu)]);

        let common = include_str!("common.wgsl");

        Self {
            smooth_x: gpu.compute(
                "Smooth_X",
                layout,
                &gpu.shader(&[common, include_str!("smooth_x.wgsl")].concat()),
            ),
            smooth_y: gpu.compute(
                "Smooth_Y",
                layout,
                &gpu.shader(&[common, include_str!("smooth_y.wgsl")].concat()),
            ),
            smooth_z: gpu.compute(
                "Smooth_Z",
                layout,
                &gpu.shader(&[common, include_str!("smooth_z.wgsl")].concat()),
            ),
            gradient: gpu.compute(
                "Gradient",
                layout,
                &gpu.shader(&[common, include_str!("gradient.wgsl")].concat()),
            ),
        }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, volume: &PhysicalVolume) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        let n_workgroups = volume.size().add(3).div(4);

        pass.set_bind_group(0, volume.binding_gradient(), &[]);

        pass.set_pipeline(&self.smooth_x);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.smooth_y);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.smooth_z);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.gradient);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
    }
}
