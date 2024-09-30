use std::any::type_name;

use wgpu::{
    CommandEncoderDescriptor, Limits, PowerPreference, RequestAdapterOptions, SubmissionIndex,
};

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

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(type_name::<Self>()),
                    required_limits: Limits {
                        max_buffer_size: 1024 * 1024 * 1024,
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

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
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
