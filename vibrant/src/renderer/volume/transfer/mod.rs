use std::ops::{Add, Div};

use wgpu::*;

use crate::{
    asset::{
        crop::CropBuffer, volume::PhysicalVolume, volume_fraction::VolumeFractionBuffer,
        volume_mask::VolumeMaskBuffer,
    },
    gpu::Gpu,
};

pub struct VolumeTransferPipeline {
    clear: ComputePipeline,
    transfer: ComputePipeline,
    copy: ComputePipeline,
    mipmap: ComputePipeline,
}

impl VolumeTransferPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            clear: gpu.compute(
                "VolumeClearPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_transfer(gpu)]),
                &gpu.shader(include_str!("clear.wgsl")),
            ),
            transfer: gpu.compute(
                "VolumeTransferPipeline",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_transfer(gpu),
                    &VolumeFractionBuffer::layout(gpu),
                    &VolumeMaskBuffer::layout(gpu),
                    &CropBuffer::layout(gpu),
                ]),
                &gpu.shader(include_str!("transfer.wgsl")),
            ),
            copy: gpu.compute(
                "VolumeCopyPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_copy(gpu)]),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
            mipmap: gpu.compute(
                "VolumeMipMapPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    /// Full volume transfer in one shot: [`Self::begin`] + [`Self::finalize`].
    /// The renderer interposes the line deposit pass between the two; callers
    /// with no line density (e.g. `benches/cascade.rs`) use this wrapper.
    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        fractions: &[VolumeFractionBuffer],
        masks: &[VolumeMaskBuffer],
        volume: &PhysicalVolume,
        crop: &CropBuffer,
    ) {
        self.begin(cmd, fractions, masks, volume, crop);
        self.finalize(cmd, volume);
    }

    /// Clears the `r32uint` accumulation textures and adds every visible volume
    /// fraction into them. After this, `LineDepositPipeline` can read-modify-write
    /// more density in before [`Self::finalize`] resolves it.
    pub fn begin(
        &self,
        cmd: &mut CommandEncoder,
        fractions: &[VolumeFractionBuffer],
        masks: &[VolumeMaskBuffer],
        volume: &PhysicalVolume,
        crop: &CropBuffer,
    ) {
        let n_workgroups = volume.size().add(3).div(4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, volume.binding_transfer(), &[]);

        pass.set_pipeline(&self.clear);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_bind_group(3, crop.binding(), &[]);

        pass.set_pipeline(&self.transfer);
        for fraction in fractions.iter().filter(|x| x.settings().visible) {
            pass.set_bind_group(1, fraction.binding(), &[]);
            pass.set_bind_group(2, masks[fraction.settings().mask].binding(), &[]);
            pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);
        }
    }

    /// Resolves the accumulated `r32uint` textures into the sampled `rgba8` +
    /// `extinction` textures and builds their mip chain.
    pub fn finalize(&self, cmd: &mut CommandEncoder, volume: &PhysicalVolume) {
        let n_workgroups = volume.size().add(3).div(4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, volume.binding_copy(), &[]);
        pass.dispatch_workgroups(n_workgroups.x, n_workgroups.y, n_workgroups.z);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = volume.size().add(3).div(4);

        for binding in volume.binding_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap.x, mipmap.y, mipmap.z);

            mipmap = mipmap.add(1).div(2);
        }
    }
}
