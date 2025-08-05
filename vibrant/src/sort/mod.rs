use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ShaderStages,
};

use crate::gpu::Gpu;

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
            size: 8,
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
            layout: &Self::layout_histogram(gpu),
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
                &gpu.pipeline_layout(&[&Self::layout_histogram(gpu), &Self::layout(gpu)]),
                &gpu.shader(
                    &(include_str!("common.wgsl").to_string()
                        + include_str!("histogram_count.wgsl")),
                ),
            ),
            histogram_sum: gpu.compute(
                "Sort::Histogram::Sum",
                &gpu.pipeline_layout(&[&Self::layout_histogram(gpu)]),
                &gpu.shader(
                    &(include_str!("common.wgsl").to_string() + include_str!("histogram_sum.wgsl")),
                ),
            ),
            scan: gpu.compute(
                "Sort::Scan",
                &gpu.pipeline_layout(&[
                    &Self::layout_histogram(gpu),
                    &RadixShift::layout(gpu),
                    &Self::layout(gpu),
                    &Self::layout(gpu),
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
    ) {
        cmd.clear_buffer(&self.histogram, 0, None);
        cmd.clear_buffer(&self.offset, 0, None);
        cmd.copy_buffer_to_buffer(count, 0, &self.count, 0, 4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        pass.set_pipeline(&self.histogram_count);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.set_bind_group(1, ping, &[]);
        pass.dispatch_workgroups(32, 1, 1);

        pass.set_pipeline(&self.histogram_sum);
        pass.dispatch_workgroups(4, 1, 1);

        pass.set_pipeline(&self.scan);
        pass.set_bind_group(1, self.shift_0.binding(), &[]);
        pass.set_bind_group(2, ping, &[]);
        pass.set_bind_group(3, pong, &[]);
        pass.dispatch_workgroups(32, 1, 1);
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Keys
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
                    // Values
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
                ],
            })
    }

    pub fn histogram(&self) -> &Buffer {
        &self.histogram
    }

    pub fn status(&self) -> &Buffer {
        &self.status
    }

    fn layout_histogram(gpu: &Gpu) -> BindGroupLayout {
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
        self.offset.destroy();
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
            usage: BufferUsages::COPY_SRC,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &SortPipeline::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: key.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: value.as_entire_binding(),
                },
            ],
        });

        Self {
            key,
            value,
            count,
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

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }
}

impl Drop for KeyValuePair {
    fn drop(&mut self) {
        self.key.destroy();
        self.value.destroy();
        self.count.destroy();
    }
}

#[cfg(test)]
pub mod test {
    use std::iter::zip;

    use itertools::Itertools;
    use pollster::FutureExt;
    use rand::Rng;

    use crate::{
        gpu::Gpu,
        sort::{KeyValuePair, SortPipeline},
    };

    #[test]
    pub fn test() {
        let gpu = &Gpu::new().block_on();

        let mut rng = rand::rng();
        let data: Vec<u32> = (0..239234).map(|_| rng.random_range(0..255)).collect();

        let expected: Vec<u32> = data.iter().copied().sorted().collect();

        let ping = KeyValuePair::new(gpu, data.len() as u32);
        let pong = KeyValuePair::new(gpu, data.len() as u32);

        gpu.queue()
            .write_buffer(ping.value(), 0, bytemuck::cast_slice(&data));

        gpu.queue().submit([]);

        let sort = SortPipeline::new(gpu);

        let mut cmd = gpu.cmd();

        sort.dispatch(&mut cmd, ping.binding(), pong.binding(), ping.count());

        gpu.submit(cmd);
        gpu.wait();

        let status: Vec<u32> = gpu.read_buffer(sort.status()).block_on();

        let histogram: Vec<u32> = gpu.read_buffer(sort.histogram()).block_on();
        println!("{:?}", &histogram[0..10]);

        let result: Vec<u32> = gpu.read_buffer(pong.value()).block_on();
        let zipped: Vec<(u32, u32)> = zip(expected, result).collect();

        assert!(zipped.iter().all(|(s, r)| s == r));
    }
}
