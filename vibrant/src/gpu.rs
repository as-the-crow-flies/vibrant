use std::{any::type_name, borrow::Cow, io, path::PathBuf};

use bytemuck::Pod;
use futures::channel::oneshot::channel;
use log::info;
use wgpu::{
    BindGroupLayout, Buffer, BufferDescriptor, BufferUsages, ColorTargetState,
    CommandEncoderDescriptor, ComputePipeline, ComputePipelineDescriptor,
    COPY_BYTES_PER_ROW_ALIGNMENT, Extent3d, Features, FragmentState, Limits, MapMode, Origin3d,
    PipelineLayout, PipelineLayoutDescriptor, PollType, PowerPreference, PrimitiveState,
    PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, Texture, TextureAspect, TextureFormat, VertexState,
};

use crate::renderer::wgsl::COMMON;

/// Round `value` up to the next multiple of `align`.
pub(crate) fn align_to(value: u32, align: u32) -> u32 {
    ((value + align - 1) / align) * align
}

/// Extract tightly-packed RGBA rows from a mapped buffer that may have padded rows (BGRA source).
///
/// The GPU copy produces rows of `bytes_per_row_padded` bytes, but only the first
/// `bytes_per_row_unpadded` bytes of each row contain pixel data. This function copies
/// those bytes row-by-row and converts each pixel from BGRA to RGBA order.
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
            output.push(pixel[2]); // R (was at index 2 in BGRA)
            output.push(pixel[1]); // G (stays)
            output.push(pixel[0]); // B (was at index 0 in BGRA)
            output.push(pixel[3]); // A (stays)
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

    /// Read a single pixel from `texture` at position (`x`, `y`).
    ///
    /// Handles wgpu row-alignment requirements (COPY_BYTES_PER_ROW_ALIGNMENT) correctly
    /// for a 1×1 copy. Returns `[R, G, B, A]` regardless of the source texture format:
    /// if the texture format is Bgra8Unorm the channels are reordered to RGBA.
    ///
    /// This is a blocking readback (map_async + poll(Wait)).
    pub async fn read_pixel(&self, texture: &Texture, x: u32, y: u32) -> [u8; 4] {
        let bytes_per_pixel = 4u32;
        let bytes_per_row_unpadded = 1 * bytes_per_pixel; // 1 pixel wide = 4 bytes
        let bytes_per_row_padded = align_to(bytes_per_row_unpadded, COPY_BYTES_PER_ROW_ALIGNMENT);

        let staging = self.device.create_buffer(&BufferDescriptor {
            label: Some("read_pixel.staging"),
            size: bytes_per_row_padded as u64,
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
                origin: Origin3d { x, y, z: 0 },
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &staging,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row_padded),
                    rows_per_image: None,
                },
            },
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([cmd.finish()]);

        let (sender, receiver) = channel();
        staging.slice(..).map_async(MapMode::Read, |r| {
            let _ = sender.send(r);
        });
        self.wait();
        receiver
            .await
            .expect("communication failed")
            .expect("buffer mapping failed");

        let view = staging.slice(..).get_mapped_range();
        // Only the first `bytes_per_row_unpadded` (4) bytes contain pixel data.
        let pixel = [view[0], view[1], view[2], view[3]];
        drop(view);
        staging.unmap();

        // Convert BGRA -> RGBA when needed
        if texture.format() == TextureFormat::Bgra8Unorm {
            [pixel[2], pixel[1], pixel[0], pixel[3]]
        } else {
            pixel
        }
    }


    /// Save the current texture to a PNG file at `path`.
    ///
    /// Returns an `io::Result` instead of panicking on filesystem or encoding errors.
    /// The texture must be `Bgra8Unorm`; the output is written as RGBA PNG.
    pub async fn save(&self, path: PathBuf, texture: &Texture) -> io::Result<()> {
        if texture.format() != TextureFormat::Bgra8Unorm {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("save: expected Bgra8Unorm texture, got {:?}", texture.format()),
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
        let mut png_writer = enc.write_header().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("PNG header error: {e}"))
        })?;
        png_writer.write_image_data(&buffer).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("PNG write error: {e}"))
        })?;

        Ok(())
    }

    /// Read back a frame as raw RGBA bytes.
    ///
    /// Returns `(data, width, height)` on success. Returns an error if the texture
    /// format is not `Bgra8Unorm`.
    pub async fn read_frame(&self, texture: &Texture) -> io::Result<(Vec<u8>, u32, u32)> {
        if texture.format() != TextureFormat::Bgra8Unorm {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("read_frame: expected Bgra8Unorm texture, got {:?}", texture.format()),
            ));
        }

        let pixel = 4u32;
        let width = texture.width();
        let height = texture.height();

        let bytes_per_row_unpadded = width * pixel;
        let bytes_per_row_padded = align_to(bytes_per_row_unpadded, COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = (bytes_per_row_padded as u64) * (height as u64);

        let result = self.device.create_buffer(&BufferDescriptor {
            label: Some("read_frame.staging"),
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

        Ok((buffer, width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_to_zero() {
        assert_eq!(align_to(0, 256), 0);
    }

    #[test]
    fn test_align_to_already_aligned() {
        assert_eq!(align_to(256, 256), 256);
        assert_eq!(align_to(512, 256), 512);
    }

    #[test]
    fn test_align_to_needs_rounding() {
        assert_eq!(align_to(1, 256), 256);
        assert_eq!(align_to(100, 256), 256);
        assert_eq!(align_to(255, 256), 256);
        assert_eq!(align_to(257, 256), 512);
    }

    #[test]
    fn test_extract_rgba_from_padded_bgra() {
        // 2x2 image, BGRA order, with 256-byte padded rows.
        // bytes_per_row_unpadded = 2 * 4 = 8
        // bytes_per_row_padded = 256
        let width = 2u32;
        let height = 2u32;
        let bytes_per_row_padded = 256u32;

        let mut mapped = vec![0u8; (bytes_per_row_padded * height) as usize];

        // Row 0, pixel 0: BGRA = (10, 20, 30, 255) -> RGBA should be (30, 20, 10, 255)
        mapped[0] = 10;  // B
        mapped[1] = 20;  // G
        mapped[2] = 30;  // R
        mapped[3] = 255; // A

        // Row 0, pixel 1: BGRA = (40, 50, 60, 200)
        mapped[4] = 40;
        mapped[5] = 50;
        mapped[6] = 60;
        mapped[7] = 200;

        // Row 1, pixel 0: BGRA = (70, 80, 90, 128) — starts at offset 256
        let row1 = bytes_per_row_padded as usize;
        mapped[row1] = 70;
        mapped[row1 + 1] = 80;
        mapped[row1 + 2] = 90;
        mapped[row1 + 3] = 128;

        // Row 1, pixel 1: BGRA = (100, 110, 120, 64)
        mapped[row1 + 4] = 100;
        mapped[row1 + 5] = 110;
        mapped[row1 + 6] = 120;
        mapped[row1 + 7] = 64;

        let result = extract_rgba_from_padded_bgra(&mapped, width, height, bytes_per_row_padded);

        // Should be 2*2*4 = 16 bytes, tightly packed RGBA
        assert_eq!(result.len(), 16);

        // Row 0, pixel 0: RGBA = (30, 20, 10, 255)
        assert_eq!(&result[0..4], &[30, 20, 10, 255]);
        // Row 0, pixel 1: RGBA = (60, 50, 40, 200)
        assert_eq!(&result[4..8], &[60, 50, 40, 200]);
        // Row 1, pixel 0: RGBA = (90, 80, 70, 128)
        assert_eq!(&result[8..12], &[90, 80, 70, 128]);
        // Row 1, pixel 1: RGBA = (120, 110, 100, 64)
        assert_eq!(&result[12..16], &[120, 110, 100, 64]);
    }

    // --- GPU integration tests (require a wgpu adapter) ---
    // Marked #[ignore] by default; run with `cargo test -- --ignored` on a GPU-capable machine.

    /// Helper: try to create a headless GPU. Returns None if no adapter is available.
    fn try_gpu() -> Option<Gpu> {
        use pollster::FutureExt;
        // Use a more lenient approach: check if adapter is available first
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::LowPower,
            ..Default::default()
        }));
        if adapter.is_err() {
            return None;
        }
        Some(Gpu::new().block_on())
    }

    #[test]
    #[ignore]
    fn test_gpu_read_pixel() {
        let gpu = match try_gpu() {
            Some(g) => g,
            None => {
                eprintln!("Skipping test_gpu_read_pixel: no GPU adapter available");
                return;
            }
        };

        // Create a 4x4 Bgra8Unorm texture
        let texture = gpu.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("test_read_pixel"),
            size: Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Write known BGRA data: pixel (2,1) = BGRA(10, 20, 30, 255)
        // We write the entire 4x4 texture with zeros, then overwrite one pixel.
        let mut data = vec![0u8; 4 * 4 * 4]; // 4x4, 4 bytes per pixel
        let offset = ((1 * 4) + 2) * 4; // row 1, col 2
        data[offset] = 10;     // B
        data[offset + 1] = 20; // G
        data[offset + 2] = 30; // R
        data[offset + 3] = 255; // A

        gpu.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * 4),
                rows_per_image: Some(4),
            },
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );

        let pixel = pollster::block_on(gpu.read_pixel(&texture, 2, 1));
        // read_pixel converts BGRA -> RGBA, so we expect (30, 20, 10, 255)
        assert_eq!(pixel, [30, 20, 10, 255]);
    }

    #[test]
    #[ignore]
    fn test_gpu_save_and_read_frame() {
        let gpu = match try_gpu() {
            Some(g) => g,
            None => {
                eprintln!("Skipping test_gpu_save_and_read_frame: no GPU adapter available");
                return;
            }
        };

        // Use 64x64 so width is already aligned to 64
        let w = 64u32;
        let h = 64u32;
        let texture = gpu.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("test_save"),
            size: Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Fill with a known BGRA color: B=50, G=100, R=150, A=255
        let pixel_data = vec![[50u8, 100, 150, 255]; (w * h) as usize];
        let flat: Vec<u8> = pixel_data.iter().flat_map(|p| p.iter().copied()).collect();

        gpu.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &flat,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        // Test save to PNG
        let dir = tempfile::tempdir().unwrap();
        let png_path = dir.path().join("test.png");
        pollster::block_on(gpu.save(png_path.clone(), &texture)).unwrap();

        assert!(png_path.exists(), "PNG file should be created");
        let meta = std::fs::metadata(&png_path).unwrap();
        assert!(meta.len() > 0, "PNG file should be non-empty");

        // Verify PNG dimensions by reading the header
        let file = std::fs::File::open(&png_path).unwrap();
        let decoder = png::Decoder::new(file);
        let reader = decoder.read_info().unwrap();
        let info = reader.info();
        assert_eq!(info.width, w);
        assert_eq!(info.height, h);

        // Test read_frame
        let (frame_data, fw, fh) = pollster::block_on(gpu.read_frame(&texture)).unwrap();
        assert_eq!(fw, w);
        assert_eq!(fh, h);
        assert_eq!(frame_data.len(), (w * h * 4) as usize);

        // Each pixel should be RGBA(150, 100, 50, 255) after BGRA->RGBA conversion
        for pixel in frame_data.chunks_exact(4) {
            assert_eq!(pixel, &[150, 100, 50, 255], "pixel RGBA mismatch");
        }
    }
}
