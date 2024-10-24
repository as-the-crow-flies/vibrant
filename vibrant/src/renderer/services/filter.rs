mod filter_binding;
mod filter_pipeline;

use bytemuck::bytes_of;
use filter_binding::FilterBinding;
use filter_pipeline::{FilterPipeline, FilterPipelineDescriptor};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, CommandEncoder, ComputePassDescriptor,
};

use crate::gpu::Gpu;

pub struct FilterDescriptor<'a> {
    pub buffer: &'a Buffer,
    pub workgroup_size: u32,
    pub items_per_thread: u32,
}

pub struct Filter {
    indirect: Buffer,
    indirect_clear: Buffer,
    binding: FilterBinding,
    pipeline: FilterPipeline,
    dispatch_count: u32,
}

impl Filter {
    pub fn new(gpu: &Gpu, descriptor: &FilterDescriptor) -> Self {
        let indirect = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("filter.indirect"),
            contents: bytes_of(&[0u32, 1, 1]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let indirect_clear = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("filter.indirect.clear"),
            contents: bytes_of(&[0u32, 1, 1]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        Self {
            binding: FilterBinding::new(gpu, descriptor.buffer, &indirect),
            pipeline: FilterPipeline::new(
                gpu,
                &FilterPipelineDescriptor {
                    workgroup_size: descriptor.workgroup_size,
                    items_per_thread: descriptor.items_per_thread,
                },
            ),
            dispatch_count: (descriptor.buffer.size() as u32)
                .div_ceil(descriptor.workgroup_size)
                .div_ceil(descriptor.items_per_thread)
                .div_ceil(4),
            indirect,
            indirect_clear,
        }
    }

    pub fn indirect(&self) -> &Buffer {
        &self.indirect
    }

    pub fn compute(&self, cmd: &mut CommandEncoder) {
        cmd.copy_buffer_to_buffer(&self.indirect_clear, 0, &self.indirect, 0, 12);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("filter.pass"),
            timestamp_writes: None,
        });

        self.pipeline
            .dispatch(&mut pass, &self.binding, self.dispatch_count);
    }
}

#[cfg(test)]
mod test {
    use itertools::Itertools;
    use pollster::FutureExt;
    use wgpu::{
        util::{BufferInitDescriptor, DeviceExt},
        BufferUsages,
    };

    use super::{Filter, FilterDescriptor};
    use crate::gpu::Gpu;

    #[test]
    fn filter_allocates_unique_index_for_every_item() {
        let gpu = Gpu::new().block_on();
        let array: Vec<u32> = (0..56241152).into_iter().map(|_| 1u32).collect();

        let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&array),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let filter = Filter::new(
            &gpu,
            &FilterDescriptor {
                buffer: &buffer,
                workgroup_size: 256,
                items_per_thread: 32,
            },
        );

        let mut cmd = gpu.cmd();
        filter.compute(&mut cmd);
        gpu.submit(cmd);
        gpu.wait();

        let result: Vec<u32> = gpu.read(&buffer).block_on();

        // Every item should have unique assigned index
        assert!(result.iter().unique().count() == array.len());
    }
}
