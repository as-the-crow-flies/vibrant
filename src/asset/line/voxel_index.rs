use std::{any::type_name, ops::Mul};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoder, ShaderStages,
};

use crate::gpu::{readback::Readback, Gpu};

/// Per-voxel segment lists: `OFFSET` (end of each voxel's range), the running
/// `OFFSET_TOTAL`, and the `INDEX` buffer the ranges point into.
pub struct VoxelIndexBuffer {
    offset: Buffer,
    offset_total: Buffer,
    index: Buffer,
    binding_write: BindGroup,
    index_buffer_size: u32,
    required_index_size: Readback<u32>,
}

impl VoxelIndexBuffer {
    pub fn new(gpu: &Gpu, resolution: u32, index_buffer_size: u32) -> Self {
        let label = Some(type_name::<Self>());

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: resolution.pow(3).mul(4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let offset_total = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let index = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: gpu
                .max_buffer_size()
                .min((index_buffer_size as u64) * 1024 * 1024),
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: offset.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: offset_total.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: index.as_entire_binding(),
                },
            ],
        });

        Self {
            offset,
            offset_total,
            index,
            binding_write,
            index_buffer_size,
            required_index_size: Readback::new(),
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.offset_total, 0, None);
    }

    pub fn count(&self) -> &Buffer {
        &self.offset_total
    }

    pub fn offset(&self) -> &Buffer {
        &self.offset
    }

    pub fn index(&self) -> &Buffer {
        &self.index
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.binding_write
    }

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        let entry = |binding| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[entry(0), entry(1), entry(2)],
            })
    }

    pub fn index_buffer_size(&self) -> u32 {
        self.index_buffer_size
    }

    /// How much index-buffer memory the current voxelization needs, as of
    /// the most recently completed readback (see
    /// [`Self::refresh_required_index_size`]). Returns the current buffer
    /// size until the first readback lands.
    pub fn required_index_size(&self) -> u32 {
        self.required_index_size
            .get()
            .unwrap_or(self.index_buffer_size)
    }

    /// Kicks off a fresh readback of the required index-buffer size. Only
    /// call this when this `VoxelIndexBuffer` isn't about to be torn down
    /// based on the result of [`Self::required_index_size`] this frame - see
    /// `Readback::refresh`'s caveat.
    pub fn refresh_required_index_size(&self, gpu: &Gpu) {
        let gpu = gpu.clone();
        let offset_total = self.offset_total.clone();

        self.required_index_size.refresh(async move {
            let bytes: u32 = gpu.read_buffer(&offset_total).await?[0];
            Some((bytes * 4).div_ceil(1024 * 1024))
        });
    }
}

impl Drop for VoxelIndexBuffer {
    fn drop(&mut self) {
        self.offset.destroy();
        self.offset_total.destroy();
        self.index.destroy();
    }
}
