use std::{any::type_name, borrow::Cow};

use wgpu::{
    CommandEncoderDescriptor, ComputePipeline, ComputePipelineDescriptor, Limits, PipelineLayout,
    PowerPreference, RequestAdapterOptions, ShaderModule, ShaderModuleDescriptor, ShaderSource,
};

use crate::renderer::constants::Constants;

pub struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Gpu {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .expect("Could not aqcuire GPU Adapter");

        let limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(type_name::<Self>()),
                    required_limits: Limits {
                        max_compute_invocations_per_workgroup: limits
                            .max_compute_invocations_per_workgroup,
                        max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
                        max_compute_workgroup_size_y: limits.max_compute_workgroup_size_y,
                        max_compute_workgroup_size_z: limits.max_compute_workgroup_size_z,
                        max_buffer_size: limits.max_buffer_size,
                        max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Could not acquire GPU Device");

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn shader(&self, source: &str, constants: Option<&Constants>) -> ShaderModule {
        self.device().create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Owned(
                constants.map(Constants::wgsl).unwrap_or_default() + source,
            )),
        })
    }

    pub fn compute(
        &self,
        layout: &PipelineLayout,
        module: &ShaderModule,
        entry_point: &str,
    ) -> ComputePipeline {
        self.device()
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(layout),
                module,
                entry_point,
                compilation_options: Default::default(),
                cache: None,
            })
    }

    pub fn cmd(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some(type_name::<Self>()),
            })
    }

    pub fn submit(&self, cmd: wgpu::CommandEncoder) {
        self.queue.submit([cmd.finish()]);
    }

    pub fn wait(&self) {
        self.device.poll(wgpu::MaintainBase::Wait);
    }
}
