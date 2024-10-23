use wgpu::*;

use crate::gpu::Gpu;

use super::{super::setting::Setting, item_type::ItemType, scan_binding::ScanBinding};

pub struct ScanPipelineDescriptor {
    pub workgroup_size: u32,
    pub item_type: ItemType,
    pub items_per_thread: u32,
}

pub struct ScanPipeline {
    pipeline: ComputePipeline,
}

impl<'a> ScanPipeline {
    pub fn new(gpu: &Gpu, descriptor: &ScanPipelineDescriptor) -> Self {
        let workgroup_size = Setting {
            name: "WORKGROUP_SIZE",
            value: descriptor.workgroup_size,
        };

        let items_per_thread = Setting {
            name: "ITEMS_PER_THREAD",
            value: descriptor.items_per_thread / 2,
        };

        let items_per_workgroup = Setting {
            name: "ITEMS_PER_WORKGROUP",
            value: descriptor.workgroup_size * 2,
        };

        let item_type = Setting {
            name: "ITEM_TYPE",
            value: descriptor.item_type,
        };

        let shader = workgroup_size.to_string()
            + &items_per_thread.to_string()
            + &items_per_workgroup.to_string()
            + &item_type.to_string()
            + include_str!("scan.wgsl");

        let module = gpu.shader(&shader, None);

        let layout = gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&ScanBinding::layout(gpu)],
                push_constant_ranges: &[],
            });

        Self {
            pipeline: gpu.compute(&layout, &module, "compute"),
        }
    }

    pub fn dispatch(&'a self, pass: &mut ComputePass<'a>, binding: &'a ScanBinding, count: u32) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &binding.binding, &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, iter::zip, ops::AddAssign};

    use bytemuck::Pod;
    use pollster::FutureExt;
    use wgpu::util::{BufferInitDescriptor, DeviceExt};

    use crate::gpu::Gpu;

    use super::*;

    #[test]
    fn can_construct_with_u32() {
        can_construct(ItemType::U32);
    }

    #[test]
    fn can_construct_with_i32() {
        can_construct(ItemType::I32);
    }

    #[test]
    fn can_construct_with_f32() {
        can_construct(ItemType::F32);
    }

    #[test]
    fn can_construct_with_uvec2() {
        can_construct(ItemType::UVEC2);
    }

    #[test]
    fn can_construct_with_uvec4() {
        can_construct(ItemType::UVEC4);
    }

    #[test]
    fn can_construct_with_ivec2() {
        can_construct(ItemType::IVEC2);
    }

    #[test]
    fn can_construct_with_ivec4() {
        can_construct(ItemType::IVEC4);
    }

    #[test]
    fn can_construct_with_fvec2() {
        can_construct(ItemType::FVEC2);
    }

    #[test]
    fn can_construct_with_fvec4() {
        can_construct(ItemType::FVEC4);
    }

    #[test]
    fn can_compute_f32() {
        can_compute::<1, f32>(ItemType::F32, 4, 4, 4);
    }

    #[test]
    fn can_compute_2f32() {
        can_compute::<2, f32>(ItemType::FVEC2, 4, 4, 4);
    }

    #[test]
    fn can_compute_4f32() {
        can_compute::<4, f32>(ItemType::FVEC4, 4, 4, 4);
    }

    #[test]
    fn can_compute_u32() {
        can_compute::<1, u32>(ItemType::U32, 4, 4, 4);
    }

    #[test]
    fn can_compute_2u32() {
        can_compute::<2, u32>(ItemType::UVEC2, 4, 4, 4);
    }

    #[test]
    fn can_compute4u32() {
        can_compute::<4, u32>(ItemType::UVEC4, 4, 4, 4);
    }

    #[test]
    fn can_compute_i32() {
        can_compute::<1, i32>(ItemType::I32, 4, 4, 4);
    }

    #[test]
    fn can_compute_2i32() {
        can_compute::<2, i32>(ItemType::IVEC2, 4, 4, 4);
    }

    #[test]
    fn can_compute_4i32() {
        can_compute::<4, i32>(ItemType::IVEC4, 4, 4, 4);
    }

    fn can_compute<const N: usize, T>(
        item_type: ItemType,
        workgroup_size: u32,
        items_per_thread: u32,
        dispatch_count: u32,
    ) where
        T: Default + Copy + AddAssign<T> + PartialEq<T> + From<u16> + Debug + Pod,
    {
        let count = (workgroup_size * items_per_thread * dispatch_count) as usize;
        let chunk_size = workgroup_size * items_per_thread;

        let input: Vec<Vec<T>> = (0..count)
            .map(|x| (0..N).map(|y| ((N * x + y) as u16).into()).collect())
            .collect();

        let expected_item: Vec<T> = input
            .chunks(chunk_size as usize)
            .flat_map(|chunk| {
                chunk.iter().scan(vec![T::default(); N], |acc, x| {
                    let result = acc.clone();

                    for (a, b) in zip(acc, x) {
                        *a += *b
                    }

                    Some(result)
                })
            })
            .flatten()
            .collect();

        let expected_sum: Vec<T> = input
            .chunks(chunk_size as usize)
            .filter_map(|chunk| {
                chunk
                    .iter()
                    .scan(vec![T::default(); N], |acc, x| {
                        for (i, v) in x.iter().enumerate() {
                            acc[i] += *v;
                        }

                        Some(acc.clone())
                    })
                    .last()
            })
            .flatten()
            .collect();

        let input: Vec<T> = input.into_iter().flatten().collect();

        // act
        let (item, sum) = compute(
            item_type,
            input,
            workgroup_size,
            items_per_thread,
            dispatch_count,
        );

        // assert
        assert_eq!(item, expected_item);
        assert_eq!(sum, expected_sum);
    }

    fn can_construct(item_type: ItemType) {
        // arrange
        let gpu = Gpu::new().block_on();

        let descriptor = ScanPipelineDescriptor {
            workgroup_size: gpu.device().limits().max_compute_workgroup_size_x,
            item_type,
            items_per_thread: 16,
        };

        // act & assert does not panic
        ScanPipeline::new(&gpu, &descriptor);
    }

    fn compute<T>(
        item_type: ItemType,
        input: Vec<T>,
        workgroup_size: u32,
        items_per_thread: u32,
        dispatch_count: u32,
    ) -> (Vec<T>, Vec<T>)
    where
        T: Pod,
    {
        // arrange
        let gpu = Gpu::new().block_on();

        let descriptor = ScanPipelineDescriptor {
            workgroup_size,
            item_type,
            items_per_thread,
        };

        let pipeline = ScanPipeline::new(&gpu, &descriptor);

        let item = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            contents: bytemuck::cast_slice(&input),
        });

        let sum = gpu.device().create_buffer(&BufferDescriptor {
            label: None,
            size: (item_type.size() * dispatch_count) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let binding = ScanBinding::new(&gpu, &item, &sum);

        // act
        let mut cmd = gpu
            .device()
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        pipeline.dispatch(&mut pass, &binding, dispatch_count);
        drop(pass);

        gpu.queue().submit([cmd.finish()]);

        let item: Vec<T> = gpu.read(&item).block_on();
        let sum: Vec<T> = gpu.read(&sum).block_on();

        (item, sum)
    }
}
