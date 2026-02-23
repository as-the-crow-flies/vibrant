use std::{any::type_name, iter::zip};

use glam::Vec4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, CommandEncoder, ComputePassDescriptor, ComputePipeline, ShaderStages,
};

use crate::{
    asset::{radiance::RadianceVolume, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct AnatomyRadiancePipeline {
    directions: DirectionBuffer,
    cascade: ComputePipeline,
    collect: ComputePipeline,
}

impl AnatomyRadiancePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("common.wgsl");

        Self {
            directions: DirectionBuffer::new(gpu),
            cascade: gpu.compute(
                "RadiancePipeline",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &DirectionBuffer::layout(gpu),
                    &RadianceVolume::layout_cascade(gpu),
                ]),
                &gpu.shader(&[common, include_str!("cascade.wgsl")].concat()),
            ),
            collect: gpu.compute(
                "RadiancePipeline",
                &gpu.pipeline_layout(&[
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &DirectionBuffer::layout(gpu),
                    &RadianceVolume::layout_write(gpu),
                ]),
                &gpu.shader(&[common, include_str!("collect.wgsl")].concat()),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        volume: &PhysicalVolume,
        radiance: &RadianceVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, volume.binding_read(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);

        for direction in &self.directions.bindings[2..3] {
            pass.set_pipeline(&self.cascade);
            pass.set_bind_group(2, direction, &[]);

            for (cascade, binding) in zip(radiance.cascades(), radiance.binding_cascades()).rev() {
                pass.set_bind_group(3, binding, &[]);
                pass.dispatch_workgroups(
                    cascade.width().div_ceil(4),
                    cascade.height().div_ceil(4),
                    cascade.depth_or_array_layers().div_ceil(4),
                );
            }

            pass.set_pipeline(&self.collect);
            pass.set_bind_group(3, radiance.binding_write(), &[]);
            pass.dispatch_workgroups(
                radiance.radiance().width().div_ceil(4),
                radiance.radiance().height().div_ceil(4),
                radiance.radiance().depth_or_array_layers().div_ceil(4),
            );
        }
    }
}

struct DirectionBuffer {
    directions: [Buffer; 6],
    bindings: Vec<BindGroup>,
}

impl DirectionBuffer {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let directions = [
            [Vec4::X, Vec4::Y, Vec4::Z],
            [Vec4::Y, Vec4::X, Vec4::Z],
            [Vec4::Z, Vec4::X, Vec4::Y],
            [Vec4::NEG_X, Vec4::Y, Vec4::Z],
            [Vec4::NEG_Y, Vec4::X, Vec4::Z],
            [Vec4::NEG_Z, Vec4::X, Vec4::Y],
        ]
        .map(|direction| {
            gpu.device().create_buffer_init(&BufferInitDescriptor {
                label,
                contents: bytemuck::bytes_of(&direction),
                usage: BufferUsages::UNIFORM,
            })
        });

        let bindings = directions
            .iter()
            .map(|buffer| {
                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout(gpu),
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer,
                            offset: 0,
                            size: None,
                        }),
                    }],
                })
            })
            .collect();

        Self {
            directions,
            bindings,
        }
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}

impl Drop for DirectionBuffer {
    fn drop(&mut self) {
        for direction in &self.directions {
            direction.destroy();
        }
    }
}
