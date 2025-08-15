use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ShaderStages,
};

use crate::gpu::Gpu;

pub struct CullPipeline {
    pipeline: ComputePipeline,
    status: Buffer,
    offset: Buffer,
    start: Buffer,
    end: Buffer,
    count: Buffer,
    binding: BindGroup,
}

impl CullPipeline {
    pub fn new(gpu: &Gpu, source: &str, layouts: &[&BindGroupLayout]) -> Self {
        let status = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 4096 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let start = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 4,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let end = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: 4,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let count = gpu.device().create_buffer(&BufferDescriptor {
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
                    resource: status.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: offset.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: start.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: end.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: count.as_entire_binding(),
                },
            ],
        });

        let pipeline = gpu.compute(
            "Cull",
            &gpu.pipeline_layout(
                &[&[&Self::layout(gpu), &Self::layout_inout(gpu)], layouts].concat(),
            ),
            &gpu.shader(&(source.to_string() + include_str!("cull.wgsl"))),
        );

        Self {
            pipeline,
            status,
            offset,
            start,
            end,
            count,
            binding,
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        inout: &BindGroup,
        start: BufferOffset,
        end: BufferOffset,
        bindings: &[&BindGroup],
    ) {
        cmd.clear_buffer(&self.status, 0, None);
        cmd.clear_buffer(&self.offset, 0, None);
        cmd.clear_buffer(&self.count, 0, None);
        cmd.copy_buffer_to_buffer(start.buffer, start.offset, &self.start, 0, 4);
        cmd.copy_buffer_to_buffer(end.buffer, end.offset, &self.end, 0, 4);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.set_bind_group(1, inout, &[]);

        for (i, &binding) in bindings.iter().enumerate() {
            pass.set_bind_group(2 + i as u32, binding, &[]);
        }

        pass.dispatch_workgroups(18, 1, 1);
    }

    pub fn count(&self) -> &Buffer {
        &self.count
    }

    pub fn layout_inout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // In
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
                    // Out
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

    fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Status
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
                    // Offset
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
                    // Start
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // End
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Count
                    BindGroupLayoutEntry {
                        binding: 4,
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
}

pub struct BufferOffset<'a> {
    pub buffer: &'a Buffer,
    pub offset: u64,
}

#[cfg(test)]
mod test {
    use std::{iter::zip, ops::Mul};

    use pollster::FutureExt;
    use rand::random;
    use wgpu::{
        util::{BufferInitDescriptor, DeviceExt},
        BindGroupDescriptor, BindGroupEntry, BufferDescriptor, BufferUsages,
    };

    use crate::{
        cull::{BufferOffset, CullPipeline},
        gpu::Gpu,
    };

    #[test]
    fn test() {
        let n = 23908239;

        let values: Vec<u32> = (0..n).map(|_| random()).collect();
        let expected: Vec<u32> = values.iter().copied().filter(|i| i % 2 == 0).collect();

        let gpu = &Gpu::new().block_on();

        let start_end_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[0u32, values.len() as u32]),
            usage: BufferUsages::COPY_SRC,
        });

        let in_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&values),
            usage: BufferUsages::STORAGE,
        });

        let out_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label: None,
            size: values.len().mul(4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let inout = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &CullPipeline::layout_inout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: in_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: out_buffer.as_entire_binding(),
                },
            ],
        });

        const SRC: &'static str = "fn cull(i: u32) -> bool { return bool(i & 1u); }";
        let pipeline = CullPipeline::new(gpu, SRC, &[]);

        let mut cmd = gpu.cmd();

        pipeline.dispatch(
            &mut cmd,
            &inout,
            BufferOffset {
                buffer: &start_end_buffer,
                offset: 0,
            },
            BufferOffset {
                buffer: &start_end_buffer,
                offset: 4,
            },
            &[],
        );

        gpu.submit(cmd);
        gpu.wait();

        let count: Vec<u32> = gpu.read_buffer(&pipeline.count()).block_on();
        assert_eq!(count[0] as usize, expected.len());

        let result: Vec<u32> = gpu.read_buffer(&out_buffer).block_on();
        let zipped: Vec<(u32, u32)> = zip(result, expected).collect();

        assert!(zipped.iter().all(|(r, e)| r == e));
    }
}
