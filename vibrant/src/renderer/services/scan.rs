mod apply_pipeline;
mod item_type;
mod scan_binding;
mod scan_pipeline;

use apply_pipeline::{ApplyPipeline, ApplyPipelineDescriptor};
pub use item_type::ItemType;
use scan_binding::ScanBinding;
use scan_pipeline::*;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoder, ComputePassDescriptor};

use crate::gpu::Gpu;

pub struct ScanDescriptor<'a> {
    pub scan: &'a Buffer,
    pub item_type: ItemType,
    pub items_per_thread: u32,
}

pub struct Scan {
    primary: ScanBinding,
    secondary: ScanBinding,
    scan: ScanPipeline,
    apply: ApplyPipeline,
    dispatch_count: u32,
}

impl Scan {
    pub fn new(gpu: &Gpu, descriptor: &ScanDescriptor) -> Self {
        let workgroup_size = gpu.device().limits().max_compute_workgroup_size_x;
        let sum_size = (descriptor.scan.size() as u32).div_ceil(workgroup_size);
        let dispatch_count = sum_size
            .div_ceil(descriptor.items_per_thread)
            .div_ceil(descriptor.item_type.size());

        let sum = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("scan.sum"),
            size: sum_size as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let total = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("scan.total"),
            size: descriptor.item_type.size() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let primary = ScanBinding::new(gpu, descriptor.scan, &sum);
        let secondary = ScanBinding::new(gpu, &sum, &total);
        let scan = ScanPipeline::new(
            gpu,
            &ScanPipelineDescriptor {
                workgroup_size,
                item_type: descriptor.item_type,
                items_per_thread: descriptor.items_per_thread,
            },
        );

        let apply = ApplyPipeline::new(
            gpu,
            &ApplyPipelineDescriptor {
                workgroup_size,
                item_type: descriptor.item_type,
                items_per_thread: descriptor.items_per_thread,
            },
        );

        Self {
            primary,
            secondary,
            scan,
            apply,
            dispatch_count,
        }
    }

    pub fn compute(&self, cmd: &mut CommandEncoder) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("scan.pass"),
            timestamp_writes: None,
        });

        self.scan
            .dispatch(&mut pass, &self.primary, self.dispatch_count);
        self.scan.dispatch(&mut pass, &self.secondary, 1);
        self.apply
            .dispatch(&mut pass, &self.primary, self.dispatch_count);
    }
}

#[cfg(test)]
mod test {
    use pollster::FutureExt;
    use wgpu::{
        util::{BufferInitDescriptor, DeviceExt},
        BufferUsages,
    };

    use crate::gpu::Gpu;

    use super::{ItemType, Scan, ScanDescriptor};

    #[test]
    fn can_scan_large_array() {
        let gpu = Gpu::new().block_on();
        let array: Vec<u32> = (0..56241152).into_iter().map(|_| 1u32).collect();

        let expected: Vec<u32> = array
            .iter()
            .scan(0, |state, &x| {
                let result = *state;
                *state += x;
                return Some(result);
            })
            .collect();

        let buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&array),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let scan = Scan::new(
            &gpu,
            &ScanDescriptor {
                scan: &buffer,
                item_type: ItemType::U32,
                items_per_thread: 32,
            },
        );

        let mut cmd = gpu.cmd();
        scan.compute(&mut cmd);
        gpu.submit(cmd);
        gpu.wait();

        let result: Vec<u32> = gpu.read(&buffer).block_on();

        assert!(expected.eq(&result))
    }
}
