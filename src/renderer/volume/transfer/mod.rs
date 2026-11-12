use std::any::type_name;

use glam::UVec3;
use wgpu::*;

use crate::{
    asset::{
        volume::{PhysicalVolume, BINS},
        volume_fraction::VolumeFractionBuffer,
        Asset,
    },
    gpu::{Gpu, Lazy},
};

/// Bakes the transfer functions after any fraction, mask or crop edit: per
/// fraction lookup tables (the tracer classifies through them per sample), the
/// tracer's majorant grid, and the cascades' extinction + mip chain.
pub struct VolumeTransferPipeline {
    lut: Lazy<ComputePipeline>,
    majorant: Lazy<ComputePipeline>,
    extinction: Lazy<ComputePipeline>,
    mipmap: Lazy<ComputePipeline>,
    lut_layout: BindGroupLayout,
    majorant_layout: BindGroupLayout,
    extinction_layout: BindGroupLayout,
    classified_layout: BindGroupLayout,
}

impl VolumeTransferPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let lut_layout = Self::lut_layout(gpu);
        let majorant_layout = Self::majorant_layout(gpu);
        let extinction_layout = PhysicalVolume::layout_write(gpu);
        let classified_layout = PhysicalVolume::layout_classified(gpu);

        let layout = lut_layout.clone();
        let lut = gpu.lazy("VolumeLutPipeline", move |gpu| {
            gpu.compute(
                "VolumeLutPipeline",
                &gpu.pipeline_layout(&[&layout, &VolumeFractionBuffer::layout(gpu)]),
                &gpu.shader("renderer/volume/transfer/lut.wgsl"),
            )
        });

        let layout = majorant_layout.clone();
        let majorant = gpu.lazy("VolumeMajorantPipeline", move |gpu| {
            gpu.compute(
                "VolumeMajorantPipeline",
                &gpu.pipeline_layout(&[&layout, &VolumeFractionBuffer::layout(gpu)]),
                &gpu.shader("renderer/volume/transfer/majorant.wgsl"),
            )
        });

        let layouts = (extinction_layout.clone(), classified_layout.clone());
        let extinction = gpu.lazy("VolumeExtinctionPipeline", move |gpu| {
            gpu.compute(
                "VolumeExtinctionPipeline",
                &gpu.pipeline_layout(&[&layouts.0, &layouts.1]),
                &gpu.shader("renderer/volume/transfer/extinction.wgsl"),
            )
        });

        let mipmap = gpu.lazy("VolumeMipMapPipeline", move |gpu| {
            gpu.compute(
                "VolumeMipMapPipeline",
                &gpu.pipeline_layout(&[&PhysicalVolume::layout_mipmap(gpu)]),
                &gpu.shader("renderer/volume/transfer/mipmap.wgsl"),
            )
        });

        Self {
            lut,
            majorant,
            extinction,
            mipmap,
            lut_layout,
            majorant_layout,
            extinction_layout,
            classified_layout,
        }
    }

    pub fn dispatch(&self, gpu: &Gpu, cmd: &mut CommandEncoder, asset: &Asset) {
        let Some(volume) = &asset.physical_volume else {
            return;
        };

        let fractions = asset.visible_volumes();

        volume.update_classify(gpu, &fractions, &asset.masks, &asset.crop);
        cmd.clear_buffer(volume.majorant(), 0, None);
        cmd.clear_buffer(volume.coarse(), 0, None);

        let slots = volume.lut_slots().iter().zip(&fractions);
        let lut: Vec<_> = slots
            .clone()
            .map(|(slot, _)| self.lut_binding(gpu, slot))
            .collect();
        let majorant: Vec<_> = slots
            .map(|(slot, _)| self.majorant_binding(gpu, volume, slot))
            .collect();

        let classified =
            volume.binding_classified(gpu, &self.classified_layout, &fractions, &asset.masks);
        let extinction = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &self.extinction_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(volume.extinction_write()),
            }],
        });

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.lut);
        for (binding, fraction) in lut.iter().zip(&fractions) {
            pass.set_bind_group(0, binding, &[]);
            pass.set_bind_group(1, fraction.binding(), &[]);
            pass.dispatch_workgroups(BINS.div_ceil(64), 1, 1);
        }

        let blocks = workgroups(PhysicalVolume::blocks(volume.size()));

        pass.set_pipeline(&self.majorant);
        for (binding, fraction) in majorant.iter().zip(&fractions) {
            pass.set_bind_group(0, binding, &[]);
            pass.set_bind_group(1, fraction.binding(), &[]);
            pass.dispatch_workgroups(blocks.x, blocks.y, blocks.z);
        }

        let voxels = workgroups(volume.size());

        pass.set_pipeline(&self.extinction);
        pass.set_bind_group(0, &extinction, &[]);
        pass.set_bind_group(1, &classified, &[]);
        pass.dispatch_workgroups(voxels.x, voxels.y, voxels.z);

        pass.set_pipeline(&self.mipmap);

        let mut size = volume.size();
        for binding in volume.binding_mipmap() {
            size = (size / 2).max(UVec3::ONE);
            let n = workgroups(size);

            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(n.x, n.y, n.z);
        }
    }

    fn lut_binding(&self, gpu: &Gpu, slot: &TextureView) -> BindGroup {
        gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &self.lut_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(slot),
            }],
        })
    }

    fn majorant_binding(
        &self,
        gpu: &Gpu,
        volume: &PhysicalVolume,
        slot: &TextureView,
    ) -> BindGroup {
        gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &self.majorant_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: volume.majorant().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(slot),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: volume.classify().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: volume.coarse().as_entire_binding(),
                },
            ],
        })
    }

    fn lut_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba16Float,
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                }],
            })
    }

    fn majorant_layout(gpu: &Gpu) -> BindGroupLayout {
        let buffer = |binding, ty| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    buffer(0, BufferBindingType::Storage { read_only: false }),
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    buffer(2, BufferBindingType::Uniform),
                    buffer(3, BufferBindingType::Storage { read_only: false }),
                ],
            })
    }
}

fn workgroups(size: UVec3) -> UVec3 {
    (size + 3) / 4
}
