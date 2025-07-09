use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, FilterMode, ShaderStages,
};

use crate::{
    asset::scalar::{R16Uint, R8Unorm, Rgba8Unorm, ScalarTexture3D},
    gpu::Gpu,
};

pub struct Density {
    density: ScalarTexture3D<R8Unorm>,
    count: ScalarTexture3D<R16Uint>,
    color: ScalarTexture3D<Rgba8Unorm>,
    density_count_buffer: Buffer,
    color_buffer: Buffer,
    binding: BindGroup,
}

impl Density {
    pub fn new(gpu: &Gpu, volume: u32) -> Self {
        let label = Some(type_name::<Self>());

        let n_voxels = volume * volume * volume;

        let density_count_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (n_voxels * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let color_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (n_voxels * 12) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &density_count_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &color_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let density = ScalarTexture3D::new(gpu, volume, FilterMode::Linear);
        let count = ScalarTexture3D::new(gpu, volume, FilterMode::Nearest);
        let color = ScalarTexture3D::new(gpu, volume, FilterMode::Linear);

        Self {
            density,
            count,
            color,
            density_count_buffer,
            color_buffer,
            binding,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.density_count_buffer, 0, None);
        cmd.clear_buffer(&self.color_buffer, 0, None);
    }

    pub fn resolution(&self) -> u32 {
        self.density.size()
    }

    pub fn density_count_buffer(&self) -> &Buffer {
        &self.density_count_buffer
    }

    pub fn color_buffer(&self) -> &Buffer {
        &self.color_buffer
    }

    pub fn density(&self) -> &ScalarTexture3D<R8Unorm> {
        &self.density
    }

    pub fn count(&self) -> &ScalarTexture3D<R16Uint> {
        &self.count
    }

    pub fn color(&self) -> &ScalarTexture3D<Rgba8Unorm> {
        &self.color
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for Density {
    fn drop(&mut self) {
        self.density_count_buffer.destroy();
        self.color_buffer.destroy();
    }
}
