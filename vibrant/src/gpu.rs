use std::{any::type_name, borrow::Cow, path::PathBuf};

use bytemuck::Pod;
use futures::channel::oneshot::channel;
use itertools::Itertools;
use wgpu::{
    BindGroupLayout, Buffer, BufferDescriptor, BufferUsages, ColorTargetState,
    CommandEncoderDescriptor, ComputePipeline, ComputePipelineDescriptor, DepthStencilState,
    Extent3d, Features, FragmentState, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, Limits,
    MapMode, MultisampleState, Origin3d, PipelineLayout, PipelineLayoutDescriptor, PowerPreference,
    PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModule, ShaderModuleDescriptor, ShaderSource, Texture,
    TextureAspect, TextureFormat,
};

use crate::renderer::constants::Constants;

pub struct Gpu {
    instance: wgpu::Instance,
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
                    required_features: Features::FLOAT32_FILTERABLE,
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Could not acquire GPU Device");

        Self {
            instance,
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
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
    }

    pub fn quad(
        &self,
        layout: &PipelineLayout,
        module: &ShaderModule,
        entry_point: &str,
        color: ColorTargetState,
        depth: Option<DepthStencilState>,
    ) -> RenderPipeline {
        self.device()
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &self.shader(include_str!("renderer/wgsl/quad.wgsl"), None),
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: depth,
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    module,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    targets: &[Some(color)],
                }),
                multiview: None,
                cache: None,
            })
    }

    pub fn pipeline_layout(&self, layouts: &[&BindGroupLayout]) -> PipelineLayout {
        self.device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: layouts,
                push_constant_ranges: &[],
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

    pub async fn read_buffer<T: Pod>(&self, buffer: &Buffer) -> Vec<T> {
        let result = self.device.create_buffer(&BufferDescriptor {
            label: Some("read.result"),
            size: buffer.size(),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut cmd = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        cmd.copy_buffer_to_buffer(buffer, 0, &result, 0, buffer.size());
        self.queue.submit([cmd.finish()]);

        return self.read(&result).await;
    }

    pub async fn read<T: Pod>(&self, buffer: &Buffer) -> Vec<T> {
        let (sender, receiver) = channel();
        buffer.slice(..).map_async(MapMode::Read, |x| {
            let _ = sender.send(x);
        });

        self.device.poll(wgpu::MaintainBase::Wait);

        receiver
            .await
            .expect("communication failed")
            .expect("buffer reading failed");

        let view = buffer.slice(..).get_mapped_range();
        return bytemuck::cast_slice(&view).to_owned();
    }

    pub async fn save(&self, path: PathBuf, texture: &Texture) {
        assert!(texture.format() == TextureFormat::Bgra8Unorm);

        let pixel = 4;
        let width = (texture.width() / 64) * 64;
        let height = texture.height();

        let result = self.device.create_buffer(&BufferDescriptor {
            label: Some("read.result"),
            size: (width * height * pixel) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut cmd = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        cmd.copy_texture_to_buffer(
            ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: Origin3d {
                    x: (texture.width() - width) / 2, // Center Crop
                    y: 0,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &result,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * pixel), // Must be multiple of 256
                    rows_per_image: None,
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([cmd.finish()]);

        let buffer = self
            .read(&result)
            .await
            .into_iter()
            .tuples()
            .flat_map(|(b, g, r, a)| [r, g, b, a])
            .collect_vec();

        let file = std::fs::File::create(path).unwrap();
        let writer = &mut std::io::BufWriter::new(file);
        let mut enc = png::Encoder::new(writer, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_source_chromaticities(png::SourceChromaticities::new(
            (0.31270, 0.32900),
            (0.64000, 0.33000),
            (0.30000, 0.60000),
            (0.15000, 0.06000),
        ));
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&buffer).unwrap();
    }
}
