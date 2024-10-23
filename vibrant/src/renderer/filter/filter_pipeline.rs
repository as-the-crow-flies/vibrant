use wgpu::*;

use crate::gpu::Gpu;

use super::{super::setting::Setting, filter_binding::FilterBinding};

pub struct FilterPipelineDescriptor {
    pub workgroup_size: u32,
    pub items_per_thread: u32,
}

pub struct FilterPipeline {
    pipeline: ComputePipeline,
}

impl<'a> FilterPipeline {
    pub fn new(gpu: &Gpu, descriptor: &FilterPipelineDescriptor) -> Self {
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

        let shader = workgroup_size.to_string()
            + &items_per_thread.to_string()
            + &items_per_workgroup.to_string()
            + include_str!("filter.wgsl");

        let module = gpu.shader(&shader, None);

        let layout = gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&FilterBinding::layout(gpu)],
                push_constant_ranges: &[],
            });

        Self {
            pipeline: gpu.compute(&layout, &module, "compute"),
        }
    }

    pub fn dispatch(&'a self, pass: &mut ComputePass<'a>, binding: &'a FilterBinding, count: u32) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &binding.binding, &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
}
