pub mod readback;

use std::{any::type_name, borrow::Cow, path::PathBuf};

use bytemuck::Pod;
use futures::channel::oneshot::channel;
use itertools::Itertools;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferDescriptor, BufferUsages, ColorTargetState, CommandEncoderDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Extent3d, Features, FragmentState, Limits, MapMode, Origin3d,
    PipelineLayout, PipelineLayoutDescriptor, PollType, PowerPreference, PrimitiveState,
    PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, Texture, TextureAspect, TextureFormat, VertexState,
};

use crate::renderer::wgsl::{COMMON, GAUSSIAN, PBR};

#[derive(Clone)]
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
            .request_device(&wgpu::DeviceDescriptor {
                label: Some(type_name::<Self>()),
                required_limits: Limits {
                    max_compute_invocations_per_workgroup: 1024,
                    max_compute_workgroup_size_x: 1024,
                    max_buffer_size: limits.max_buffer_size,
                    max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
                    max_compute_workgroup_storage_size: 32768,
                    max_storage_buffers_per_shader_stage: 10,
                    max_sampled_textures_per_shader_stage: 48,
                    max_storage_textures_per_shader_stage: 8,
                    ..Default::default()
                },
                required_features: Features::FLOAT32_FILTERABLE | Features::SUBGROUP,
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

    pub fn max_buffer_size(&self) -> u64 {
        self.device()
            .limits()
            .max_storage_buffer_binding_size
            .min(self.device().limits().max_buffer_size)
    }

    pub fn shader(&self, source: &str) -> ShaderModule {
        let enable = if cfg!(target_arch = "wasm32") {
            "enable subgroups;\n"
        } else {
            ""
        };

        self.device().create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Owned(
                enable.to_string() + COMMON + PBR + GAUSSIAN + source,
            )),
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

    pub fn binding(
        &self,
        label: &str,
        layout: &BindGroupLayout,
        entries: Vec<BindingResource>,
    ) -> BindGroup {
        self.device().create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &entries
                .into_iter()
                .enumerate()
                .map(|(binding, resource)| BindGroupEntry {
                    binding: binding as u32,
                    resource: resource,
                })
                .collect_vec(),
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
                multiview_mask: None,
                cache: None,
            })
    }

    pub fn pipeline_layout(&self, layouts: &[&BindGroupLayout]) -> PipelineLayout {
        self.device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &layouts.iter().map(|&layout| Some(layout)).collect_vec(),
                immediate_size: 0,
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

    /// Copies `buffer` to a staging buffer and reads it back. Returns `None`
    /// if the buffer was destroyed before the readback could complete - e.g.
    /// a caller tore down the owning resource while a background readback
    /// (see [`readback::Readback`]) was still in flight. Callers should
    /// treat that as "try again later", not an error.
    pub async fn read_buffer<T: Pod>(&self, buffer: &Buffer) -> Option<Vec<T>> {
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

    /// See [`Self::read_buffer`] for the meaning of `None`.
    pub async fn read<T: Pod>(&self, buffer: &Buffer) -> Option<Vec<T>> {
        let (sender, receiver) = channel();
        buffer.slice(..).map_async(MapMode::Read, |x| {
            let _ = sender.send(x);
        });

        self.wait();

        receiver.await.ok()?.ok()?;

        let view = buffer.slice(..).get_mapped_range().ok()?;
        Some(bytemuck::cast_slice(&view).to_owned())
    }

    pub async fn save(&self, path: PathBuf, texture: &Texture) {
        assert!(texture.format() == TextureFormat::Rgba8Unorm);

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
            TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: Origin3d {
                    x: (texture.width() - width) / 2, // Center Crop
                    y: 0,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &result,
                layout: TexelCopyBufferLayout {
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

        let Some(buffer) = self.read::<u8>(&result).await else {
            log::warn!("screenshot: source texture was destroyed before it could be read back");
            return;
        };

        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, width, height);
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

        Self::deliver(path, png_bytes);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn deliver(path: PathBuf, png_bytes: Vec<u8>) {
        std::fs::write(path, png_bytes).expect("failed to write screenshot");
    }

    /// Browsers have no filesystem to write to, so instead we hand the
    /// browser a `Blob` and drive a synthetic `<a download>` click - the
    /// standard way to trigger a file download from script.
    #[cfg(target_arch = "wasm32")]
    fn deliver(path: PathBuf, png_bytes: Vec<u8>) {
        use wasm_bindgen::JsCast;
        use web_sys::{Blob, BlobPropertyBag, Url};

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("screenshot.png");

        let array = js_sys::Uint8Array::from(png_bytes.as_slice());
        let parts = js_sys::Array::new();
        parts.push(&array);

        let options = BlobPropertyBag::new();
        options.set_type("image/png");

        let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)
            .expect("failed to create screenshot blob");

        let url = Url::create_object_url_with_blob(&blob).expect("failed to create object url");

        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("no document");
        let anchor = document
            .create_element("a")
            .expect("failed to create anchor")
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .expect("created element was not an anchor");
        anchor.set_href(&url);
        anchor.set_download(filename);

        let body = document.body().expect("no document body");
        body.append_child(&anchor).expect("failed to attach anchor");
        anchor.click();
        body.remove_child(&anchor).expect("failed to detach anchor");

        let _ = Url::revoke_object_url(&url);
    }
}
