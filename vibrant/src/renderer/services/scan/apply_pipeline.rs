use wgpu::*;

use super::{super::setting::Setting, item_type::ItemType, scan_binding::ScanBinding};
use crate::gpu::Gpu;

pub struct ApplyPipelineDescriptor {
    pub workgroup_size: u32,
    pub item_type: ItemType,
    pub items_per_thread: u32,
}

pub struct ApplyPipeline {
    pipeline: ComputePipeline,
}

impl<'a> ApplyPipeline {
    pub fn new(gpu: &Gpu, descriptor: &ApplyPipelineDescriptor) -> Self {
        let workgroup_size = Setting {
            name: "WORKGROUP_SIZE",
            value: descriptor.workgroup_size,
        };

        let items_per_thread = Setting {
            name: "ITEMS_PER_THREAD",
            value: descriptor.items_per_thread / 2,
        };

        let item_type = Setting {
            name: "ITEM_TYPE",
            value: descriptor.item_type,
        };

        let shader = workgroup_size.to_string()
            + &items_per_thread.to_string()
            + &item_type.to_string()
            + include_str!("apply.wgsl");

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
    use std::{
        fmt::Debug,
        ops::{Add, AddAssign},
    };

    use bytemuck::Pod;
    use pollster::FutureExt;
    use wgpu::util::{BufferInitDescriptor, DeviceExt};

    use super::*;

    #[test]
    fn can_construct_with_u32() {
        can_construct(ItemType::U32)
    }

    #[test]
    fn can_construct_with_i32() {
        can_construct(ItemType::I32)
    }

    #[test]
    fn can_construct_with_f32() {
        can_construct(ItemType::F32)
    }

    #[test]
    fn can_construct_with_uvec2() {
        can_construct(ItemType::UVEC2)
    }

    #[test]
    fn can_construct_with_uvec4() {
        can_construct(ItemType::UVEC4)
    }

    #[test]
    fn can_construct_with_ivec2() {
        can_construct(ItemType::IVEC2)
    }

    #[test]
    fn can_construct_with_ivec4() {
        can_construct(ItemType::IVEC4)
    }

    #[test]
    fn can_construct_with_fvec2() {
        can_construct(ItemType::FVEC2)
    }

    #[test]
    fn can_construct_with_fvec4() {
        can_construct(ItemType::FVEC4)
    }

    #[test]
    fn can_compute_f32() {
        can_compute::<1, f32>(ItemType::F32, 4, 4, 4)
    }

    #[test]
    fn can_compute_2f32() {
        can_compute::<2, f32>(ItemType::FVEC2, 4, 4, 4)
    }

    #[test]
    fn can_compute_4f32() {
        can_compute::<4, f32>(ItemType::FVEC4, 4, 4, 4)
    }

    #[test]
    fn can_compute_u32() {
        can_compute::<1, u32>(ItemType::U32, 4, 4, 4)
    }

    #[test]
    fn can_compute_2u32() {
        can_compute::<2, u32>(ItemType::UVEC2, 4, 4, 4)
    }

    #[test]
    fn can_compute4u32() {
        can_compute::<4, u32>(ItemType::UVEC4, 4, 4, 4)
    }

    #[test]
    fn can_compute_i32() {
        can_compute::<1, i32>(ItemType::I32, 4, 4, 4)
    }

    #[test]
    fn can_compute_2i32() {
        can_compute::<2, i32>(ItemType::IVEC2, 4, 4, 4)
    }

    #[test]
    fn can_compute_4i32() {
        can_compute::<4, i32>(ItemType::IVEC4, 4, 4, 4)
    }

    fn can_construct(item_type: ItemType) {
        // arrange
        let gpu = Gpu::new().block_on();

        let descriptor = ApplyPipelineDescriptor {
            workgroup_size: gpu.device().limits().max_compute_workgroup_size_x,
            item_type,
            items_per_thread: 16,
        };

        // act & assert does not panic
        ApplyPipeline::new(&gpu, &descriptor);
    }

    fn can_compute<const N: usize, T>(
        item_type: ItemType,
        workgroup_size: u32,
        items_per_thread: u32,
        dispatch_count: u32,
    ) where
        T: Default
            + Copy
            + AddAssign<T>
            + Add<T, Output = T>
            + PartialEq<T>
            + From<u16>
            + Debug
            + Pod,
    {
        // arrange
        let input_count = (workgroup_size * items_per_thread * dispatch_count) as usize;
        let sum_count = (items_per_thread * dispatch_count) as usize;
        let chunk_size = (workgroup_size * items_per_thread) as usize;

        let input: Vec<Vec<T>> = (0..input_count)
            .map(|x| (0..N).map(|y| ((N * x + y) as u16).into()).collect())
            .collect();

        let sum: Vec<Vec<T>> = (0..sum_count)
            .map(|x| (0..N).map(|y| ((N * x + y) as u16).into()).collect())
            .collect();

        let expected_item: Vec<T> = input
            .chunks(chunk_size)
            .zip(&sum)
            .flat_map(|(chunk, sum)| {
                chunk
                    .iter()
                    .flat_map(move |item| item.iter().zip(sum.clone()).map(|(a, b)| *a + b))
            })
            .collect();

        let expected_sum: Vec<T> = sum
            .iter()
            .flat_map(|item| item.iter().map(|&x| x))
            .collect();

        let input: Vec<T> = input.into_iter().flatten().collect();
        let sum: Vec<T> = sum
            .iter()
            .flat_map(|item| item.iter().map(|&x| x))
            .collect();

        // act
        let (item, sum) = compute(
            item_type,
            input,
            sum,
            workgroup_size,
            items_per_thread,
            dispatch_count,
        );

        // assert
        assert_eq!(item, expected_item);
        assert_eq!(sum, expected_sum);
    }

    fn compute<T>(
        item_type: ItemType,
        input: Vec<T>,
        sum: Vec<T>,
        workgroup_size: u32,
        items_per_thread: u32,
        dispatch_count: u32,
    ) -> (Vec<T>, Vec<T>)
    where
        T: Pod,
    {
        // arrange
        let gpu = Gpu::new().block_on();

        let descriptor = ApplyPipelineDescriptor {
            workgroup_size,
            item_type,
            items_per_thread,
        };

        let pipeline = ApplyPipeline::new(&gpu, &descriptor);

        let item = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            contents: bytemuck::cast_slice(&input),
        });

        let sum = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            contents: bytemuck::cast_slice(&sum),
        });

        let binding = ScanBinding::new(&gpu, &item, &sum);

        // act
        let mut cmd = gpu.cmd();

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        pipeline.dispatch(&mut pass, &binding, dispatch_count);
        drop(pass);

        gpu.submit(cmd);

        let item: Vec<T> = gpu.read(&item).block_on();
        let sum: Vec<T> = gpu.read(&sum).block_on();

        (item, sum)
    }
}
