use std::{any::type_name, borrow::Cow, io, path::PathBuf};

use bytemuck::Pod;
use futures::channel::oneshot::channel;
use log::info;
use wgpu::{
    BindGroupLayout, Buffer, BufferDescriptor, BufferUsages, ColorTargetState,
    CommandEncoderDescriptor, ComputePipeline, ComputePipelineDescriptor, Extent3d, Features,
    FragmentState, Limits, MapMode, Origin3d, PipelineLayout, PipelineLayoutDescriptor, PollType,
    PowerPreference, PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModule, ShaderModuleDescriptor, ShaderSource, TexelCopyBufferInfo,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureFormat,
    VertexState, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use crate::renderer::wgsl::COMMON;

/// Round `value` up to the next multiple of `align`.
pub(crate) fn align_to(value: u32, align: u32) -> u32 {
    ((value + align - 1) / align) * align
}

/// Extract tightly-packed RGBA rows from a mapped buffer that may have padded rows (BGRA source).
pub(crate) fn extract_rgba_from_padded_bgra(
    mapped: &[u8],
    width: u32,
    height: u32,
    bytes_per_row_padded: u32,
) -> Vec<u8> {
    let bytes_per_row_unpadded = width * 4;
    let mut output = Vec::with_capacity((width * height * 4) as usize);

    for row in 0..height {
        let src_start = (row * bytes_per_row_padded) as usize;
        let src_end = src_start + bytes_per_row_unpadded as usize;
        let row_bytes = &mapped[src_start..src_end];

        // Reorder each pixel from BGRA to RGBA
        for pixel in row_bytes.chunks_exact(4) {
            output.push(pixel[2]);
            output.push(pixel[1]);
            output.push(pixel[0]);
            output.push(pixel[3]);
        }
    }

    output
}

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

        info!("{:?}", adapter.features());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some(type_name::<Self>()),
                required_limits: Limits {
                    max_buffer_size: limits.max_buffer_size,
                    max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
                    max_storage_buffers_per_shader_stage: limits
                        .max_storage_buffers_per_shader_stage,
                    ..Default::default()
                },
                required_features: Features::FLOAT32_FILTERABLE
                    | Features::ADDRESS_MODE_CLAMP_TO_BORDER,
                ..Default::default()
            })
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

    pub fn shader(&self, source: &str) -> ShaderModule {
        self.device().create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Owned(COMMON.to_string() + source)),
        })
    }

    pub fn compute(
        &self,
        label: &str,
        layout: &PipelineLayout,
        module: &ShaderModule,
    ) -> ComputePipeline {
        self.device()
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                module,
                entry_point: None,
                compilation_options: Default::default(),
                cache: None,
            })
    }

    pub fn quad(
        &self,
        label: &str,
        layout: &PipelineLayout,
        target: ColorTargetState,
        module: &ShaderModule,
    ) -> RenderPipeline {
        self.device()
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: VertexState {
                    module,
                    entry_point: Some("vertex"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                fragment: Some(FragmentState {
                    module,
                    entry_point: Some("fragment"),
                    targets: &[Some(target)],
                    compilation_options: Default::default(),
                }),
                multisample: Default::default(),
                depth_stencil: None,
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

    pub fn wait(&self) -> bool {
        self.device
            .poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .is_ok()
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

        self.wait();

        receiver
            .await
            .expect("communication failed")
            .expect("buffer reading failed");

        let view = buffer.slice(..).get_mapped_range();
        return bytemuck::cast_slice(&view).to_owned();
    }

    pub async fn save(&self, path: PathBuf, texture: &Texture) -> io::Result<()> {
        if texture.format() != TextureFormat::Bgra8Unorm {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "save: expected Bgra8Unorm texture, got {:?}",
                    texture.format()
                ),
            ));
        }

        let pixel = 4u32;
        let width = texture.width();
        let height = texture.height();

        let bytes_per_row_unpadded = width * pixel;
        let bytes_per_row_padded = align_to(bytes_per_row_unpadded, COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = (bytes_per_row_padded as u64) * (height as u64);

        let result = self.device.create_buffer(&BufferDescriptor {
            label: Some("save.staging"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut cmd = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        cmd.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &result,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row_padded),
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

        // Map the staging buffer and extract RGBA pixels row-by-row,
        // handling potential padding between rows and converting BGRA -> RGBA.
        let (sender, receiver) = channel();
        result.slice(..).map_async(MapMode::Read, |x| {
            let _ = sender.send(x);
        });
        self.wait();
        receiver
            .await
            .expect("communication failed")
            .expect("buffer mapping failed");

        let mapped = result.slice(..).get_mapped_range();
        let buffer = extract_rgba_from_padded_bgra(&mapped, width, height, bytes_per_row_padded);
        drop(mapped);
        result.unmap();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(&path)?;
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
        let mut png_writer = enc
            .write_header()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PNG header error: {e}")))?;
        png_writer
            .write_image_data(&buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PNG write error: {e}")))?;

        Ok(())
    }
}
