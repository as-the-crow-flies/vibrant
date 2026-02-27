use std::ops::{Add, Div};

use glam::UVec3;
use wgpu::*;

use crate::{asset::volume::PhysicalVolume, gpu::Gpu};

pub struct GaussianPipeline {
    gaussian_x: ComputePipeline,
    gaussian_y: ComputePipeline,
    gaussian_z: ComputePipeline,
    copy: ComputePipeline,
}

impl GaussianPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = &gpu.pipeline_layout(&[
            &PhysicalVolume::layout_texture(gpu),
            &PhysicalVolume::layout_texture(gpu),
        ]);

        let x = "const DIRECTION: vec3<u32> = vec3<u32>(1, 0, 0);\n";
        let y = "const DIRECTION: vec3<u32> = vec3<u32>(0, 1, 0);\n";
        let z = "const DIRECTION: vec3<u32> = vec3<u32>(0, 0, 1);\n";

        Self {
            gaussian_x: gpu.compute(
                "Gaussian_X",
                layout,
                &gpu.shader(&[x, include_str!("gaussian.wgsl")].concat()),
            ),
            gaussian_y: gpu.compute(
                "Gaussian_Y",
                layout,
                &gpu.shader(&[y, include_str!("gaussian.wgsl")].concat()),
            ),
            gaussian_z: gpu.compute(
                "Gaussian_Z",
                layout,
                &gpu.shader(&[z, include_str!("gaussian.wgsl")].concat()),
            ),
            copy: gpu.compute(
                "Gaussian_Copy",
                layout,
                &gpu.shader(&[z, include_str!("copy.wgsl")].concat()),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        ping: &BindGroup,
        pong: &BindGroup,
        size: UVec3,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        let n_workgroups = size.add(3).div(4);

        pass.set_pipeline(&self.gaussian_x);
        pass.set_bind_group(0, ping, &[]);
        pass.set_bind_group(1, pong, &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.gaussian_y);
        pass.set_bind_group(0, pong, &[]);
        pass.set_bind_group(1, ping, &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.gaussian_z);
        pass.set_bind_group(0, ping, &[]);
        pass.set_bind_group(1, pong, &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, pong, &[]);
        pass.set_bind_group(1, ping, &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
    }
}
