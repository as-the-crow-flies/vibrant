use std::{
    any::type_name,
    ops::{Add, Div},
};

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::{occupancy::OccupancyBuffer, LineBuffer},
        texture::{MipTexture3D, R32Float},
        volume::PhysicalVolume,
    },
    gpu::Gpu,
    renderer::environment::Environment,
};

/// Writes neutral line density (from the occupancy pyramid) into the shared
/// `PhysicalVolume`'s `line_extinction` texture (mip 0), then builds its mip
/// chain. `cascade.wgsl` folds that into its occlusion so the radiance
/// cascade(s) light lines and volume together; the volume tracer never marches
/// it.
pub struct LineDepositPipeline {
    deposit: ComputePipeline,
    mipmap: ComputePipeline,
}

impl LineDepositPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            deposit: gpu.compute(
                type_name::<Self>(),
                &gpu.pipeline_layout(&[
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &OccupancyBuffer::layout_read(gpu),
                ]),
                &gpu.shader(include_str!("deposit.wgsl")),
            ),
            mipmap: gpu.compute(
                "LineDeposit::MipMap",
                &gpu.pipeline_layout(&[&MipTexture3D::<R32Float>::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("../occupancy/mipmap.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        _environment: &Environment,
        volume: &PhysicalVolume,
        line: &LineBuffer,
    ) {
        let n = volume.size().add(3).div(4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("LineDeposit"),
            ..Default::default()
        });

        pass.set_pipeline(&self.deposit);
        pass.set_bind_group(0, volume.line_extinction().binding_write(), &[]);
        pass.set_bind_group(1, line.occupancy().binding(), &[]);
        pass.dispatch_workgroups(n.x, n.y, n.z);

        drop(pass);

        volume.line_extinction().mipmap(cmd, &self.mipmap);
    }
}
