use std::{any::type_name, mem::replace};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, CompareFunction,
    CompositeAlphaMode, DepthBiasState, DepthStencilState, Extent3d, PresentMode, ShaderStages,
    StencilState, SurfaceConfiguration, SurfaceTarget, SurfaceTexture, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use super::gpu::Gpu;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    depth: Texture,
    visibility: Buffer,
}

impl Surface {
    pub const COLOR_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
    pub const COLOR_SRGB_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth16Unorm;

    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let (width, height) = (1, 1);

        surface.configure(gpu.device(), &Self::configuration(width, height));

        Self {
            surface,
            depth: Self::create_depth_texture(gpu, width, height),
            visibility: Self::create_visibility_buffer(gpu, width, height),
        }
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.surface
            .configure(gpu.device(), &Self::configuration(width, height));

        replace(
            &mut self.depth,
            Self::create_depth_texture(gpu, width, height),
        )
        .destroy();

        replace(
            &mut self.visibility,
            Self::create_visibility_buffer(gpu, width, height),
        )
        .destroy();
    }

    pub fn surface_frame(&self, gpu: &Gpu) -> SurfaceFrame {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Could not optain SurfaceTexture");

        SurfaceFrame {
            frame: Frame::new(gpu, &surface_texture.texture, &self.depth, &self.visibility),
            surface_texture,
        }
    }

    pub fn color_target() -> ColorTargetState {
        ColorTargetState {
            format: Surface::COLOR_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn color_srgb_target() -> ColorTargetState {
        ColorTargetState {
            format: Surface::COLOR_SRGB_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn depth_target() -> DepthStencilState {
        DepthStencilState {
            format: Surface::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }
    }

    pub fn create_color_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
        gpu.device().create_texture(&TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Surface::COLOR_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[Surface::COLOR_FORMAT, Surface::COLOR_SRGB_FORMAT],
        })
    }

    pub fn create_depth_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
        gpu.device().create_texture(&TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[Self::DEPTH_FORMAT],
        })
    }

    pub fn create_visibility_buffer(gpu: &Gpu, width: u32, height: u32) -> Buffer {
        gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: (width * height * 4) as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    pub fn configuration(width: u32, height: u32) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: Self::COLOR_FORMAT,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![Self::COLOR_FORMAT, Self::COLOR_SRGB_FORMAT],
        }
    }
}

pub struct SurfaceFrame {
    frame: Frame,
    surface_texture: SurfaceTexture,
}

impl SurfaceFrame {
    pub fn present(self) {
        self.surface_texture.present();
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }
}

pub struct Frame {
    width: u32,
    height: u32,
    color: TextureView,
    color_srgb: TextureView,
    depth: TextureView,
    visibility: BindGroup,
}

impl Frame {
    pub fn new(gpu: &Gpu, color: &Texture, depth: &Texture, visibility: &Buffer) -> Self {
        let label = Some(type_name::<Self>());

        Frame {
            width: color.width(),
            height: color.height(),
            color: color.create_view(&TextureViewDescriptor {
                label,
                format: Some(Surface::COLOR_FORMAT),
                ..Default::default()
            }),
            color_srgb: color.create_view(&TextureViewDescriptor {
                label,
                format: Some(Surface::COLOR_SRGB_FORMAT),
                ..Default::default()
            }),
            depth: depth.create_view(&TextureViewDescriptor {
                label,
                format: Some(Surface::DEPTH_FORMAT),
                ..Default::default()
            }),
            visibility: gpu.device().create_bind_group(&BindGroupDescriptor {
                label,
                layout: &gpu
                    .device()
                    .create_bind_group_layout(&BindGroupLayoutDescriptor {
                        label: Some(type_name::<Self>()),
                        entries: &[BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }],
                    }),
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: visibility,
                        offset: 0,
                        size: None,
                    }),
                }],
            }),
        }
    }

    pub fn test(gpu: &Gpu, width: u32, height: u32) -> Self {
        let color = Surface::create_color_texture(gpu, width, height);
        let depth = Surface::create_depth_texture(gpu, width, height);
        let visibility = Surface::create_visibility_buffer(gpu, width, height);

        Self::new(gpu, &color, &depth, &visibility)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn color(&self) -> &TextureView {
        &self.color
    }

    pub fn color_srgb(&self) -> &TextureView {
        &self.color_srgb
    }

    pub fn depth(&self) -> &TextureView {
        &self.depth
    }

    pub fn visibility(&self) -> &BindGroup {
        &self.visibility
    }
}
