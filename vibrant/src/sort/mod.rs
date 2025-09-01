use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ShaderStages,
};

use crate::gpu::Gpu;

#[derive(PartialEq, Eq)]
pub enum SortPipelineRadix {
    R8,
    R16,
    R24,
    R32,
}

pub struct SortPipeline {
    histogram: Buffer,
    status: Buffer,
    offset: Buffer,
    count: Buffer,
    binding: BindGroup,
    shift_0: RadixShift,
    shift_8: RadixShift,
    shift_16: RadixShift,
    shift_24: RadixShift,
    histogram_count: ComputePipeline,
    histogram_sum: ComputePipeline,
    scan: ComputePipeline,
}

impl SortPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let histogram = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 1024 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let status = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 256 * 4096 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 20,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let count = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: histogram.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: status.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: offset.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: count.as_entire_binding(),
                },
            ],
        });

        Self {
            histogram,
            status,
            offset,
            count,
            binding,
            shift_0: RadixShift::new(gpu, 0),
            shift_8: RadixShift::new(gpu, 8),
            shift_16: RadixShift::new(gpu, 16),
            shift_24: RadixShift::new(gpu, 24),
            histogram_count: gpu.compute(
                "Sort::Histogram::Count",
                &gpu.pipeline_layout(&[&Self::layout(gpu), &KeyValuePair::layout(gpu)]),
                &gpu.shader(
                    &(include_str!("common.wgsl").to_string()
                        + include_str!("histogram_count.wgsl")),
                ),
            ),
            histogram_sum: gpu.compute(
                "Sort::Histogram::Sum",
                &gpu.pipeline_layout(&[&Self::layout(gpu)]),
                &gpu.shader(
                    &(include_str!("common.wgsl").to_string() + include_str!("histogram_sum.wgsl")),
                ),
            ),
            scan: gpu.compute(
                "Sort::Scan",
                &gpu.pipeline_layout(&[
                    &Self::layout(gpu),
                    &RadixShift::layout(gpu),
                    &KeyValuePair::layout(gpu),
                    &KeyValuePair::layout(gpu),
                ]),
                &gpu.shader(&(include_str!("common.wgsl").to_string() + include_str!("scan.wgsl"))),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        ping: &BindGroup,
        pong: &BindGroup,
        count: &Buffer,
        radix: SortPipelineRadix,
    ) {
        cmd.clear_buffer(&self.histogram, 0, None);
        cmd.clear_buffer(&self.offset, 0, None);
        cmd.copy_buffer_to_buffer(count, 0, &self.count, 0, 4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        let n_workgroups = 18;

        pass.set_pipeline(&self.histogram_count);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.set_bind_group(1, ping, &[]);
        pass.dispatch_workgroups(n_workgroups, 1, 1);

        pass.set_pipeline(&self.histogram_sum);
        pass.dispatch_workgroups(4, 1, 1);

        pass.set_pipeline(&self.scan);

        pass.set_bind_group(1, self.shift_0.binding(), &[]);
        pass.set_bind_group(2, ping, &[]);
        pass.set_bind_group(3, pong, &[]);
        pass.dispatch_workgroups(n_workgroups, 1, 1);

        if radix == SortPipelineRadix::R8 {
            return;
        }

        pass.set_bind_group(1, self.shift_8.binding(), &[]);
        pass.set_bind_group(2, pong, &[]);
        pass.set_bind_group(3, ping, &[]);
        pass.dispatch_workgroups(n_workgroups, 1, 1);

        if radix == SortPipelineRadix::R16 {
            return;
        }

        pass.set_bind_group(1, self.shift_16.binding(), &[]);
        pass.set_bind_group(2, ping, &[]);
        pass.set_bind_group(3, pong, &[]);
        pass.dispatch_workgroups(n_workgroups, 1, 1);

        if radix == SortPipelineRadix::R24 {
            return;
        }

        pass.set_bind_group(1, self.shift_24.binding(), &[]);
        pass.set_bind_group(2, pong, &[]);
        pass.set_bind_group(3, ping, &[]);
        pass.dispatch_workgroups(n_workgroups, 1, 1);
    }

    pub fn histogram(&self) -> &Buffer {
        &self.histogram
    }

    pub fn status(&self) -> &Buffer {
        &self.status
    }

    fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Histogram
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Status
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Offset
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Count
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
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

impl Drop for SortPipeline {
    fn drop(&mut self) {
        self.histogram.destroy();
        self.status.destroy();
        self.offset.destroy();
        self.count.destroy();
    }
}

struct RadixShift {
    buffer: Buffer,
    binding: BindGroup,
}

impl RadixShift {
    pub fn new(gpu: &Gpu, shift: u32) -> Self {
        let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: Some(type_name::<Self>()),
            contents: bytes_of(&shift),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self { buffer, binding }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
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

impl Drop for RadixShift {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

pub struct KeyValuePair {
    key: Buffer,
    value: Buffer,
    count: Buffer,
    offset: Buffer,
    binding: BindGroup,
}

impl KeyValuePair {
    pub fn new(gpu: &Gpu, size: u32) -> Self {
        let key = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: (size * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let value = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: (size * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let count = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: Some(type_name::<Self>()),
            contents: bytes_of(&size),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: key.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: value.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: count.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: offset.as_entire_binding(),
                },
            ],
        });

        Self {
            key,
            value,
            count,
            offset,
            binding,
        }
    }

    pub fn key(&self) -> &Buffer {
        &self.key
    }

    pub fn value(&self) -> &Buffer {
        &self.value
    }

    pub fn count(&self) -> &Buffer {
        &self.count
    }

    pub fn offset(&self) -> &Buffer {
        &self.offset
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn clear_count(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.count, 0, None);
    }

    pub fn clear_offset(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.offset, 0, None);
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Keys
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Values
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Count
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Offset
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: BindingType::Buffer {
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

impl Drop for KeyValuePair {
    fn drop(&mut self) {
        self.key.destroy();
        self.value.destroy();
        self.count.destroy();
        self.offset.destroy();
    }
}

#[cfg(test)]
pub mod test {
    use std::iter::zip;

    use itertools::Itertools;
    use pollster::FutureExt;

    use crate::{
        gpu::Gpu,
        sort::{KeyValuePair, SortPipeline, SortPipelineRadix},
    };

    #[test]
    pub fn test() {
        fn quick_random(seed: &mut u32) -> u32 {
            // Parameters from Numerical Recipes
            *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *seed
        }

        let mut seed = 123456789;

        let n = 1024 * 1024;

        let keys: Vec<u32> = (0..n).collect();
        let values: Vec<u32> = (0..n).map(|_| quick_random(&mut seed)).collect();

        let expected_values: Vec<u32> = values.iter().copied().sorted().collect();
        let expected_keys: Vec<u32> = values
            .iter()
            .copied()
            .enumerate()
            .sorted_by_key(|&(_, v)| v)
            .map(|(k, _)| k as u32)
            .collect();

        let gpu = &Gpu::new().block_on();
        let ping = KeyValuePair::new(gpu, values.len() as u32);
        let pong = KeyValuePair::new(gpu, values.len() as u32);

        gpu.queue()
            .write_buffer(ping.key(), 0, bytemuck::cast_slice(&keys));
        gpu.queue()
            .write_buffer(ping.value(), 0, bytemuck::cast_slice(&values));

        gpu.queue().submit([]);

        let sort = SortPipeline::new(gpu);

        let mut cmd = gpu.cmd();

        sort.dispatch(
            &mut cmd,
            ping.binding(),
            pong.binding(),
            ping.count(),
            SortPipelineRadix::R32,
        );

        gpu.submit(cmd);
        gpu.wait();

        let resulting_keys: Vec<u32> = gpu.read_buffer(ping.key()).block_on();
        let resulting_values: Vec<u32> = gpu.read_buffer(ping.value()).block_on();

        let zipped_keys: Vec<(u32, u32)> = zip(expected_keys, resulting_keys).collect();
        let zipped_values: Vec<(u32, u32)> = zip(expected_values, resulting_values).collect();

        assert!(zipped_keys.iter().all(|(s, r)| s == r));
        assert!(zipped_values.iter().all(|(s, r)| s == r));
    }
}
