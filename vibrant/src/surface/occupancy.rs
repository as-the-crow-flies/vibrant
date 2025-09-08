use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, CommandEncoder, FilterMode, ShaderStages,
};

use crate::{
    asset::texture::{MipTexture3D, R32Float, R32Uint},
    gpu::Gpu,
};

pub struct OccupancyBuffer {
    density: MipTexture3D<R32Float>,
    count: MipTexture3D<R32Uint>,
    occupancy_count_buffer: Buffer,
    binding_write: BindGroup,
    binding_read: BindGroup,
}

impl OccupancyBuffer {
    pub fn new(gpu: &Gpu, resolution: u32) -> Self {
        let label = Some(type_name::<Self>());

        let n_voxels = resolution * resolution * resolution;

        let occupancy_count_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (n_voxels * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let density =
            MipTexture3D::new(gpu, resolution, resolution, resolution, FilterMode::Linear);
        let count = MipTexture3D::new(gpu, resolution, resolution, resolution, FilterMode::Nearest);

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries: &[density.binding_entries(0), count.binding_entries(2)].concat(),
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &occupancy_count_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            density,
            count,
            occupancy_count_buffer,
            binding_read,
            binding_write,
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.occupancy_count_buffer, 0, None);
    }

    pub fn resolution(&self) -> u32 {
        self.density.resolution()
    }

    pub fn occupancy_count_buffer(&self) -> &Buffer {
        &self.occupancy_count_buffer
    }

    pub fn pyramid(&self) -> &MipTexture3D<R32Float> {
        &self.density
    }

    pub fn count(&self) -> &MipTexture3D<R32Uint> {
        &self.count
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.binding_write
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    MipTexture3D::<R32Float>::layout_entries(0),
                    MipTexture3D::<R32Uint>::layout_entries(2),
                ]
                .concat(),
            })
    }

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}

impl Drop for OccupancyBuffer {
    fn drop(&mut self) {
        self.occupancy_count_buffer.destroy();
    }
}
