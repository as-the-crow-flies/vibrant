pub mod color;
pub mod culling;
pub mod occlusion;
pub mod occupancy;
pub mod ui;

use std::any::type_name;

use bytemuck::bytes_of;
use color::ColorBuffer;
use log::{info, warn};
use occlusion::OcclusionBuffer;
use occupancy::OccupancyBuffer;
use ui::UiBuffer;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder,
    CompositeAlphaMode, PresentMode, RenderPassDescriptor, RenderPipeline, ShaderStages,
    SurfaceCapabilities, SurfaceConfiguration, SurfaceTarget, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

use crate::{
    asset::texture::{MipTexture3D, R32Float, R32Uint},
    controller::settings::Settings,
    surface::culling::CullingBuffer,
};

use super::gpu::Gpu;

pub struct Frame {
    color: ColorBuffer,
    post: ColorBuffer,
    ui: UiBuffer,
    bloom_a: ColorBuffer,
    bloom_b: ColorBuffer,
    occupancy: OccupancyBuffer,
    occlusion: OcclusionBuffer,
    culling: CullingBuffer,
    binding: BindGroup,
}

impl Frame {
    pub fn new(gpu: &Gpu, settings: &Settings) -> Self {
        let color = ColorBuffer::new(gpu, settings.width, settings.height);
        let post = ColorBuffer::new(gpu, settings.width, settings.height);
        let ui = UiBuffer::new(gpu, settings.width, settings.height);
        let bloom_a = ColorBuffer::new(gpu, settings.width, settings.height);
        let bloom_b = ColorBuffer::new(gpu, settings.width, settings.height);

        let occupancy = OccupancyBuffer::new(gpu, settings.volume);
        let occlusion = OcclusionBuffer::new(gpu, settings.volume);
        let culling = CullingBuffer::new(gpu, settings.volume);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[
                occupancy.pyramid().binding_entries(0),
                occupancy.count().binding_entries(2),
                occlusion.ambient().binding_entries(4),
                occlusion.directional().binding_entries(6),
            ]
            .concat(),
        });

        Self {
            color,
            post,
            ui,
            bloom_a,
            bloom_b,
            occupancy,
            occlusion,
            culling,
            binding,
        }
    }

    pub fn color(&self) -> &ColorBuffer {
        &self.color
    }

    pub fn post(&self) -> &ColorBuffer {
        &self.post
    }

    pub fn ui(&self) -> &UiBuffer {
        &self.ui
    }

    pub fn bloom_a(&self) -> &ColorBuffer {
        &self.bloom_a
    }

    pub fn bloom_b(&self) -> &ColorBuffer {
        &self.bloom_b
    }

    pub fn occupancy(&self) -> &OccupancyBuffer {
        &self.occupancy
    }

    pub fn occlusion(&self) -> &OcclusionBuffer {
        &self.occlusion
    }

    pub fn culling(&self) -> &CullingBuffer {
        &self.culling
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    MipTexture3D::<R32Float>::layout_entries(0), // Occupancy - Density
                    MipTexture3D::<R32Uint>::layout_entries(2),  // Occupancy - Count
                    MipTexture3D::<R32Float>::layout_entries(4), // Occlusion - Ambient
                    MipTexture3D::<R32Float>::layout_entries(6), // Occlusion - Directional
                ]
                .concat(),
            })
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    format: TextureFormat,
    sdr_format: TextureFormat,
    hdr_format: Option<TextureFormat>,
    hdr_supported: bool,
    hdr_output: bool,
    hdr_params_layout: BindGroupLayout,
    hdr_params_buffer: Buffer,
    hdr_params_binding: BindGroup,
    display_hdr: RenderPipeline,
    display_sdr: RenderPipeline,
    buffer: Frame,
}

impl Surface {
    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let caps = surface.get_capabilities(gpu.adapter());
        let (sdr_format, hdr_format) = Self::detect_formats(&caps);

        let format = sdr_format;
        let hdr_supported = hdr_format.is_some();
        let hdr_output = false;

        let hdr_params_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("Surface::HdrParams"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let hdr_params_layout = Self::hdr_params_layout(gpu);

        let hdr_params_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some("Surface::HdrParams::Binding"),
            layout: &hdr_params_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &hdr_params_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let display_hdr = gpu.quad(
            "Surface::Display::HDR",
            &gpu.pipeline_layout(&[
                &ColorBuffer::layout(gpu),
                &UiBuffer::layout(gpu),
                &hdr_params_layout,
            ]),
            ColorTargetState {
                format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::all(),
            },
            &gpu.shader(include_str!("display_hdr.wgsl")),
        );

        let display_sdr = gpu.quad(
            "Surface::Display::SDR",
            &gpu.pipeline_layout(&[&ColorBuffer::layout(gpu), &UiBuffer::layout(gpu)]),
            ColorTargetState {
                format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::all(),
            },
            &gpu.shader(include_str!("display_sdr.wgsl")),
        );

        surface.configure(gpu.device(), &Self::config(1, 1, format));

        info!(
            "Surface formats: sdr={:?}, hdr={:?}; active={:?}, hdr_output={}",
            sdr_format, hdr_format, format, hdr_output
        );

        Self {
            surface,
            format,
            sdr_format,
            hdr_format,
            hdr_supported,
            hdr_output,
            hdr_params_layout,
            hdr_params_buffer,
            hdr_params_binding,
            display_hdr,
            display_sdr,
            buffer: Frame::new(gpu, &Settings::new()),
        }
    }

    pub fn maybe_resize(&mut self, gpu: &Gpu, settings: &Settings) -> &Self {
        if settings.width == self.buffer.color().width()
            && settings.height == self.buffer.color().height()
            && settings.volume == self.buffer.occupancy().resolution()
        {
            return self;
        }

        self.buffer = Frame::new(gpu, &settings);
        self.surface
            .configure(gpu.device(), &Self::config(settings.width, settings.height, self.format));

        self
    }

    pub fn update_output_mode(&mut self, gpu: &Gpu, settings: &Settings, prefer_hdr_output: bool) {
        // Keep HDR display parameters in a dedicated uniform for final present pass.
        gpu.queue().write_buffer(
            &self.hdr_params_buffer,
            0,
            bytes_of(&[
                settings.hdr_paper_white_nits,
                settings.hdr_peak_nits,
                0.0,
                0.0,
            ]),
        );

        let wants_hdr = prefer_hdr_output && self.hdr_supported;
        let desired_format = if wants_hdr {
            self.hdr_format.unwrap_or(self.sdr_format)
        } else {
            self.sdr_format
        };

        if self.hdr_output == wants_hdr && self.format == desired_format {
            return;
        }

        self.hdr_output = wants_hdr;
        self.format = desired_format;

        self.display_hdr = gpu.quad(
            "Surface::Display::HDR",
            &gpu.pipeline_layout(&[
                &ColorBuffer::layout(gpu),
                &UiBuffer::layout(gpu),
                &self.hdr_params_layout,
            ]),
            ColorTargetState {
                format: self.format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::all(),
            },
            &gpu.shader(include_str!("display_hdr.wgsl")),
        );

        self.display_sdr = gpu.quad(
            "Surface::Display::SDR",
            &gpu.pipeline_layout(&[&ColorBuffer::layout(gpu), &UiBuffer::layout(gpu)]),
            ColorTargetState {
                format: self.format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::all(),
            },
            &gpu.shader(include_str!("display_sdr.wgsl")),
        );

        self.surface
            .configure(gpu.device(), &Self::config(settings.width, settings.height, self.format));

        info!(
            "Output mode updated: hdr_output={}, format={:?}",
            self.hdr_output, self.format
        );
    }

    pub fn present(&self, gpu: &Gpu, mut cmd: CommandEncoder) {
        if let Some(surface) = self.surface.get_current_texture().ok() {
            let view = surface.texture.create_view(&TextureViewDescriptor::default());

            {
                let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Surface::Present"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });

                if self.hdr_output {
                    pass.set_pipeline(&self.display_hdr);
                } else {
                    pass.set_pipeline(&self.display_sdr);
                }

                pass.set_bind_group(0, self.buffer.post().binding(), &[]);
                pass.set_bind_group(1, self.buffer.ui().binding(), &[]);
                if self.hdr_output {
                    pass.set_bind_group(2, &self.hdr_params_binding, &[]);
                }
                pass.draw(0..4, 0..1);
            }

            gpu.submit(cmd);
            surface.present();

            if !gpu.wait() {
                warn!("Could not poll GPU")
            }
        } else {
            warn!("Could not obtain surface texture");
        }
    }

    fn config(width: u32, height: u32, format: TextureFormat) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![format],
        }
    }

    fn detect_formats(caps: &SurfaceCapabilities) -> (TextureFormat, Option<TextureFormat>) {
        let hdr_format = caps
            .formats
            .contains(&TextureFormat::Rgba16Float)
            .then_some(TextureFormat::Rgba16Float);

        let sdr_format = caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| {
                caps.formats
                    .iter()
                    .copied()
                    .find(|format| *format != TextureFormat::Rgba16Float)
            })
            .unwrap_or(caps.formats[0]);

        (sdr_format, hdr_format)
    }

    fn hdr_params_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Surface::HdrParams::Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }

    pub fn buffer(&self) -> &Frame {
        &self.buffer
    }

    pub fn hdr_output(&self) -> bool {
        self.hdr_output
    }

    pub fn hdr_supported(&self) -> bool {
        self.hdr_supported
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }
}
