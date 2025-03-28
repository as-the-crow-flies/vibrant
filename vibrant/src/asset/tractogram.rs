use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::{any::type_name, f32::consts::PI};

use glam::{Mat4, Quat, Vec3, Vec4};

use itertools::Itertools;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, ShaderStages,
};

use crate::{file, gpu::Gpu};

use super::filter::Filter;

pub struct Tractogram {
    vertices: Buffer,
    binding: BindGroup,
    filter_default: Filter,
    filter_culling: Filter,
}

impl Tractogram {
    pub fn vertices(&self) -> &Buffer {
        &self.vertices
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn vertex_count(&self) -> u32 {
        (self.vertices.size() / 12) as u32
    }

    pub fn filter_default(&self) -> &Filter {
        &self.filter_default
    }

    pub fn filter_culling(&self) -> &Filter {
        &self.filter_culling
    }

    pub fn new(gpu: &Gpu, tck: &file::Tck) -> Self {
        let label = Some(type_name::<Self>());

        let mut indices = tck
            .vertices()
            .iter()
            .enumerate()
            .filter_map(|(index, &vertex)| vertex.is_finite().then_some(index as u32))
            .collect_vec();

        let vertices = tck
            .vertices()
            .iter()
            .map(|v| Vec4::new(v.x, v.y, v.z, if v.is_finite() { 1.0 } else { 0.0 }))
            .collect_vec();

        let scale = tck.bounds().scale();
        let transform = Mat4::from_scale_rotation_translation(
            Vec3::new(scale, scale, scale),
            Quat::from_rotation_x(0.5 * PI),
            Vec3::ZERO,
        );

        fn morton_encode(x: u8, y: u8, z: u8) -> u32 {
            fn part1by2(n: u8) -> u32 {
                let mut n = n as u32;
                n = (n | (n << 16)) & 0x030000FF;
                n = (n | (n << 8)) & 0x0300F00F;
                n = (n | (n << 4)) & 0x030C30C3;
                n = (n | (n << 2)) & 0x09249249;
                n
            }
            part1by2(x) | (part1by2(y) << 1) | (part1by2(z) << 2)
        }

        let morton: Vec<u32> = vertices
            .par_iter()
            .map(|&vertex| {
                let vertex = (u8::MAX as f32) * (0.5 + 0.5 * vertex / scale);
                morton_encode(vertex.x as u8, vertex.y as u8, vertex.z as u8)
            })
            .collect();

        indices.par_sort_by_cached_key(|&index| morton[index as usize]);

        let world_to_tractogram = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
        });

        let tractogram_to_world = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&transform.inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let vertices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &tractogram_to_world,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &world_to_tractogram,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &vertices,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            filter_default: Filter::new(gpu, &indices),
            filter_culling: Filter::new(gpu, &indices),
            vertices,
            binding,
        }
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for Tractogram {
    fn drop(&mut self) {
        self.vertices.destroy();
    }
}
