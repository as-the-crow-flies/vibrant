use std::{any::type_name, cell::RefCell, rc::Rc};

use wgpu::{include_wgsl, ComputePipeline, ComputePipelineDescriptor, PipelineLayoutDescriptor};

use crate::{asset_buffer::tractogram::Tractogram, gpu::Gpu};

use super::camera::Camera;

pub struct ComputeTractogramRenderer {
    tractogram: Rc<RefCell<Tractogram>>,
    pipeline: ComputePipeline,
}

impl ComputeTractogramRenderer {
    pub fn new(gpu: &Gpu, camera: &Camera, tractogram: Rc<RefCell<Tractogram>>) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(include_wgsl!("wgsl/tractogram_compute.wgsl"));

        let layout = gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[&camera.layout, &Tractogram::bind_group_layout(gpu)],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device()
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label,
                layout: Some(&layout),
                module: &module,
                entry_point: "main",
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            tractogram,
            pipeline,
        }
    }
}
