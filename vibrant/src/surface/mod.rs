pub mod color;
pub mod culling;
pub mod occlusion;
pub mod occupancy;
pub mod ui;

use crate::controller::settings::RecordingMode;
use crate::renderer::record::Recorder;
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
    controller::settings::{AntiAliasingMode, Settings},
    surface::culling::CullingBuffer,
};

use super::gpu::Gpu;

pub struct Frame {
    color: ColorBuffer,
    post: ColorBuffer,
    aa: ColorBuffer,
    ui: UiBuffer,
    bloom_a: ColorBuffer,
    bloom_b: ColorBuffer,
    smaa_edges: ColorBuffer,
    smaa_blend: ColorBuffer,
    taa_history: ColorBuffer,
    occupancy: OccupancyBuffer,
    occlusion: OcclusionBuffer,
    culling: CullingBuffer,
    binding: BindGroup,
    // Foveated rendering buffers
    foveated_peripheral: Option<ColorBuffer>,
    foveated_focus: Option<ColorBuffer>,
}

impl Frame {
    pub fn new(gpu: &Gpu, settings: &Settings) -> Self {
        match settings.recording_mode {
            RecordingMode::Performance => Self::new_performance(gpu, settings),
            RecordingMode::Quality => Self::new_quality(gpu, settings),
        }
    }

    fn new_performance(gpu: &Gpu, settings: &Settings) -> Self {
        let color = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);
        let post = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);
        let aa = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);
        let ui = UiBuffer::new(gpu, settings.width, settings.height);
        let bloom_a = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);
        let bloom_b = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);
        let smaa_edges = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);
        let smaa_blend = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);
        let taa_history = ColorBuffer::new_sdr(gpu, settings.render_width, settings.render_height);

        let foveated_peripheral = if settings.foveated {
            Some(ColorBuffer::new_sdr(
                gpu,
                settings.peripheral_width(),
                settings.peripheral_height(),
            ))
        } else {
            None
        };
        let foveated_focus = if settings.foveated {
            Some(ColorBuffer::new_sdr(
                gpu,
                settings.focus_width(),
                settings.focus_height(),
            ))
        } else {
            None
        };

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
            aa,
            ui,
            bloom_a,
            bloom_b,
            smaa_edges,
            smaa_blend,
            taa_history,
            occupancy,
            occlusion,
            culling,
            binding,
            foveated_peripheral,
            foveated_focus,
        }
    }

    fn new_quality(gpu: &Gpu, settings: &Settings) -> Self {
        let color = ColorBuffer::new(gpu, settings.render_width, settings.render_height);
        let post = ColorBuffer::new(gpu, settings.render_width, settings.render_height);
        let aa = ColorBuffer::new(gpu, settings.render_width, settings.render_height);
        let ui = UiBuffer::new(gpu, settings.width, settings.height);
        let bloom_a = ColorBuffer::new(gpu, settings.render_width, settings.render_height);
        let bloom_b = ColorBuffer::new(gpu, settings.render_width, settings.render_height);
        let smaa_edges = ColorBuffer::new(gpu, settings.render_width, settings.render_height);
        let smaa_blend = ColorBuffer::new(gpu, settings.render_width, settings.render_height);
        let taa_history = ColorBuffer::new(gpu, settings.render_width, settings.render_height);

        let foveated_peripheral = if settings.foveated {
            Some(ColorBuffer::new(
                gpu,
                settings.peripheral_width(),
                settings.peripheral_height(),
            ))
        } else {
            None
        };
        let foveated_focus = if settings.foveated {
            Some(ColorBuffer::new(
                gpu,
                settings.focus_width(),
                settings.focus_height(),
            ))
        } else {
            None
        };

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
            aa,
            ui,
            bloom_a,
            bloom_b,
            smaa_edges,
            smaa_blend,
            taa_history,
            occupancy,
            occlusion,
            culling,
            binding,
            foveated_focus,
            foveated_peripheral,
        }
    }

    pub fn color(&self) -> &ColorBuffer {
        &self.color
    }
    pub fn post(&self) -> &ColorBuffer {
        &self.post
    }
    pub fn aa(&self) -> &ColorBuffer {
        &self.aa
    }
    pub fn smaa_edges(&self) -> &ColorBuffer {
        &self.smaa_edges
    }
    pub fn smaa_blend(&self) -> &ColorBuffer {
        &self.smaa_blend
    }
    pub fn taa_history(&self) -> &ColorBuffer {
        &self.taa_history
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

    pub fn foveated_focus(&self) -> Option<&ColorBuffer> {
        self.foveated_focus.as_ref()
    }

    pub fn foveated_peripheral(&self) -> Option<&ColorBuffer> {
        self.foveated_peripheral.as_ref()
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
    capture_pipeline_sdr: RenderPipeline,
    capture_pipeline_hdr: RenderPipeline,
    buffer: Frame,
    // cached frame when Adaptive AA is enabled
    // when camera switch from still to motion, AA switch from SSAA to TAA/SMAA
    // use frame_cache to prevent stutter
    frame_cache: Option<Frame>,
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

        let capture_pipeline_sdr = gpu.quad(
            "Surface::Capture",
            &gpu.pipeline_layout(&[&ColorBuffer::layout(gpu), &UiBuffer::layout(gpu)]),
            ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::all(),
            },
            &gpu.shader(include_str!("display_sdr.wgsl")),
        );

        let capture_pipeline_hdr = gpu.quad(
            "Surface::Capture::HDR",
            &gpu.pipeline_layout(&[
                &ColorBuffer::layout(gpu),
                &UiBuffer::layout(gpu),
                &hdr_params_layout,
            ]),
            ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::all(),
            },
            &gpu.shader(include_str!("display_hdr.wgsl")),
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
            capture_pipeline_sdr,
            capture_pipeline_hdr,
            buffer: Frame::new(gpu, &Settings::new()),
            frame_cache: None,
        }
    }

    pub fn maybe_resize(&mut self, gpu: &Gpu, settings: &Settings) -> &Self {
        let foveated_matches = match (&self.buffer.foveated_focus, &self.buffer.foveated_peripheral, settings.foveated) {
            (Some(focus), Some(peripheral), true) => {
                focus.width() == settings.focus_width()
                    && focus.height() == settings.focus_height()
                    && peripheral.width() == settings.peripheral_width()
                    && peripheral.height() == settings.peripheral_height()
            }
            (None, None, false) => true,
            _ => false,
        };

        if settings.width == self.buffer.ui().width()
            && settings.height == self.buffer.ui().height()
            && settings.render_width == self.buffer.color().width()
            && settings.render_height == self.buffer.color().height()
            && settings.volume == self.buffer.occupancy().resolution()
            && foveated_matches
        {
            // dimensions unchanged, free the cache when not in Adaptive mode
            if settings.aa_mode != AntiAliasingMode::Adaptive {
                self.frame_cache = None;
            }
            return self;
        }

        // check if the cached frame matches the requested dimensions
        // this happens during Adaptive AA switching between SSAA and TAA/SMAA
        let cache_hit = self.frame_cache.as_ref().is_some_and(|cached| {
            settings.render_width == cached.color().width()
                && settings.render_height == cached.color().height()
                && settings.width == cached.ui().width()
                && settings.height == cached.ui().height()
                && settings.volume == cached.occupancy().resolution()
        });

        if cache_hit {
            // swap current ↔ cached: instant, no GPU allocation.
            let cached = self.frame_cache.take().unwrap();
            let old = std::mem::replace(&mut self.buffer, cached);
            if settings.aa_mode == AntiAliasingMode::Adaptive {
                self.frame_cache = Some(old);
            }
        } else if settings.aa_mode == AntiAliasingMode::Adaptive {
            // cache miss but in Adaptive mode
            // save the current frame before allocating a new one
            let old = std::mem::replace(&mut self.buffer, Frame::new(gpu, settings));
            self.frame_cache = Some(old);
        } else {
            // not in Adaptive mode — just reallocate, no caching operation
            self.buffer = Frame::new(gpu, settings);
            self.frame_cache = None;
        }

        self.surface.configure(
            gpu.device(),
            &Self::config(settings.width, settings.height, self.format),
        );

        self
    }

    pub fn update_output_mode(&mut self, gpu: &Gpu, settings: &Settings, prefer_hdr_output: bool) {
        gpu.queue().write_buffer(
            &self.hdr_params_buffer,
            0,
            bytes_of(&[
                settings.hdr_paper_white_nits,
                settings.hdr_peak_nits,
                0.0f32,
                0.0f32,
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

        // Re-detect supported formats only when the HDR preference is actually
        // changing, not every frame
        let caps = self.surface.get_capabilities(gpu.adapter());
        let (sdr_format, hdr_format) = Self::detect_formats(&caps);
        self.sdr_format = sdr_format;
        self.hdr_format = hdr_format;
        self.hdr_supported = hdr_format.is_some();

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

        self.surface.configure(
            gpu.device(),
            &Self::config(settings.width, settings.height, self.format),
        );

        info!(
            "Output mode updated: hdr_output={}, format={:?}",
            self.hdr_output, self.format
        );
    }

    pub fn present(
        &self,
        gpu: &Gpu,
        mut cmd: CommandEncoder,
        recorder: &mut Option<Recorder>,
        recording_mode: RecordingMode,
    ) {
        if let Some(surface_tex) = self.surface.get_current_texture().ok() {
            let view = surface_tex
                .texture
                .create_view(&TextureViewDescriptor::default());

            // Normal display pass
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
                pass.set_bind_group(0, self.buffer.aa().binding(), &[]);
                pass.set_bind_group(1, self.buffer.ui().binding(), &[]);
                if self.hdr_output {
                    pass.set_bind_group(2, &self.hdr_params_binding, &[]);
                }
                pass.draw(0..4, 0..1);
            }

            // Capture pass — only when recording
            if let Some(rec) = recorder.as_mut() {
                let cap_w = self.buffer.ui().width();
                let cap_h = self.buffer.ui().height();

                match recording_mode {
                    RecordingMode::Performance => {
                        // Performance: run through capture pipeline (applies Reinhard, same as display)
                        // then read from Bgra8Unorm — matches what's on screen
                        let cap_w = self.buffer.ui().width();
                        let cap_h = self.buffer.ui().height();

                        let capture_tex = gpu.device().create_texture(&wgpu::TextureDescriptor {
                            label: Some("Surface::Capture"),
                            size: wgpu::Extent3d {
                                width: cap_w,
                                height: cap_h,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::COPY_SRC,
                            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
                        });

                        let capture_view =
                            capture_tex.create_view(&TextureViewDescriptor::default());

                        {
                            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                                label: Some("Surface::Capture::Performance"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &capture_view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                ..Default::default()
                            });

                            // Use capture_pipeline (display_sdr.wgsl) — applies Reinhard, matches screen
                            pass.set_pipeline(&self.capture_pipeline_sdr);
                            pass.set_bind_group(0, self.buffer.post().binding(), &[]); // post for performance
                            pass.set_bind_group(1, self.buffer.ui().binding(), &[]);
                            pass.draw(0..4, 0..1);
                        }

                        let staging =
                            Self::read_capture_raw(gpu, &mut cmd, &capture_tex, cap_w, cap_h, 4);

                        gpu.submit(cmd);
                        surface_tex.present();
                        gpu.wait();

                        staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                        gpu.wait();

                        let data = staging.slice(..).get_mapped_range();
                        let bytes_per_row = ((cap_w * 4 + 255) / 256) * 256;
                        let mut pixels = Vec::with_capacity((cap_w * cap_h * 4) as usize);
                        for row in 0..cap_h {
                            let start = (row * bytes_per_row) as usize;
                            let end = start + (cap_w * 4) as usize;
                            pixels.extend_from_slice(&data[start..end]);
                        }
                        drop(data);
                        rec.write_frame(&pixels);
                    }

                    RecordingMode::Quality => {
                        // Quality: render aa buffer through capture pipeline
                        // into Bgra8Unorm — GPU applies tone mapping
                        let cap_w = self.buffer.ui().width();
                        let cap_h = self.buffer.ui().height();

                        let capture_tex = gpu.device().create_texture(&wgpu::TextureDescriptor {
                            label: Some("Surface::Capture::HDR"),
                            size: wgpu::Extent3d {
                                width: cap_w,
                                height: cap_h,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba16Float, // 16-bit float
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::COPY_SRC,
                            view_formats: &[],
                        });

                        let capture_view =
                            capture_tex.create_view(&TextureViewDescriptor::default());

                        {
                            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                                label: Some("Surface::Capture::HDR"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &capture_view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                ..Default::default()
                            });

                            // Use HDR capture pipeline — outputs raw HDR values, no normalization
                            pass.set_pipeline(&self.capture_pipeline_hdr);
                            pass.set_bind_group(0, self.buffer.aa().binding(), &[]);
                            pass.set_bind_group(1, self.buffer.ui().binding(), &[]);
                            pass.set_bind_group(2, &self.hdr_params_binding, &[]);
                            pass.draw(0..4, 0..1);
                        }

                        // 8 bytes per pixel for Rgba16Float
                        let staging =
                            Self::read_capture_raw(gpu, &mut cmd, &capture_tex, cap_w, cap_h, 8);

                        gpu.submit(cmd);
                        surface_tex.present();
                        gpu.wait();

                        staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                        gpu.wait();

                        let data = staging.slice(..).get_mapped_range();

                        // Convert Rgba16Float → rgba64le (16-bit unsigned integer)
                        // FFmpeg expects rgba64le: 4 channels x 16-bit unsigned per pixel
                        let bytes_per_row = ((cap_w * 8 + 255) / 256) * 256;
                        let mut pixels = Vec::with_capacity((cap_w * cap_h * 8) as usize);
                        for row in 0..cap_h {
                            let row_start = (row * bytes_per_row) as usize;
                            for col in 0..cap_w {
                                let px = row_start + (col * 8) as usize;

                                // Read 4 x f16 values
                                let r = f16_to_f32(u16::from_le_bytes([data[px], data[px + 1]]));
                                let g =
                                    f16_to_f32(u16::from_le_bytes([data[px + 2], data[px + 3]]));
                                let b =
                                    f16_to_f32(u16::from_le_bytes([data[px + 4], data[px + 5]]));
                                let a =
                                    f16_to_f32(u16::from_le_bytes([data[px + 6], data[px + 7]]));

                                // Convert to 16-bit unsigned integer (0-65535)
                                let r16 = ((r.max(0.0) * 65535.0) as u32).min(65535) as u16;
                                let g16 = ((g.max(0.0) * 65535.0) as u32).min(65535) as u16;
                                let b16 = ((b.max(0.0) * 65535.0) as u32).min(65535) as u16;
                                let a16 = ((a.max(0.0) * 65535.0) as u32).min(65535) as u16;

                                // Write as little-endian 16-bit values
                                pixels.extend_from_slice(&r16.to_le_bytes());
                                pixels.extend_from_slice(&g16.to_le_bytes());
                                pixels.extend_from_slice(&b16.to_le_bytes());
                                pixels.extend_from_slice(&a16.to_le_bytes());
                            }
                        }
                        drop(data);
                        rec.write_frame(&pixels);
                    }
                }

                if !gpu.wait() {
                    warn!("Could not poll GPU");
                }
                return;
            }

            gpu.submit(cmd);
            surface_tex.present();

            if !gpu.wait() {
                warn!("Could not poll GPU");
            }
        } else {
            warn!("Could not obtain surface texture");
        }
    }

    fn read_capture_raw(
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
    ) -> wgpu::Buffer {
        let bytes_per_row = ((width * bytes_per_pixel + 255) / 256) * 256;

        let staging = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Surface::Capture::Staging"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        cmd.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        staging
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

fn f16_to_f32(bits: u16) -> f32 {
    let exp = ((bits >> 10) & 0x1f) as i32;
    let mant = (bits & 0x3ff) as u32;
    let sign = if bits >> 15 == 1 { -1.0f32 } else { 1.0f32 };

    sign * if exp == 0 {
        mant as f32 * 2.0f32.powi(-24)
    } else if exp == 31 {
        if mant == 0 {
            f32::INFINITY
        } else {
            f32::NAN
        }
    } else {
        (mant as f32 / 1024.0 + 1.0) * 2.0f32.powi(exp - 15)
    }
}
