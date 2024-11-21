use std::{any::type_name, fmt::Debug, iter::repeat, num::NonZeroU64};

use bytemuck::{cast_slice, NoUninit, Pod};
use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, ComputePass, ShaderStages,
};

use crate::gpu::Gpu;

pub trait PushData: NoUninit + Pod + Debug {}
impl PushData for u32 {}
impl PushData for i32 {}
impl PushData for f32 {}

pub struct Push {
    stride: u32,
    buffer: Buffer,
    binding: BindGroup,
}

impl Push {
    pub fn new<T: PushData>(gpu: &Gpu, data: &[T]) -> Self {
        let stride = gpu.device().limits().min_uniform_buffer_offset_alignment;
        let label = Some(type_name::<Self>());

        let contents = data
            .into_iter()
            .flat_map(|&element| repeat(element).take((stride / 4) as usize))
            .collect_vec();

        let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: cast_slice(&contents),
            usage: BufferUsages::UNIFORM,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: NonZeroU64::new(4),
                }),
            }],
        });

        Self {
            stride,
            buffer,
            binding,
        }
    }

    pub fn len(&self) -> u32 {
        self.buffer.size() as u32 / self.stride
    }

    pub fn apply(&self, pass: &mut ComputePass, index: u32, offset: u32) {
        pass.set_bind_group(index, &self.binding, &[offset * self.stride]);
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}
