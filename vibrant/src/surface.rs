use std::any::type_name;

use wgpu::{
    ColorTargetState, ColorWrites, CompareFunction, CompositeAlphaMode, DepthBiasState,
    DepthStencilState, Extent3d, PresentMode, StencilFaceState, StencilState, SurfaceCapabilities,
    SurfaceConfiguration, SurfaceTarget, SurfaceTexture, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use super::gpu::Gpu;

pub struct Frame<'a> {
    surface: SurfaceTexture,
    depth: &'a Texture,
}

pub struct TestFrame {
    color: Texture,
    depth: Texture,
}

impl TestFrame {
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        Self {
            color: gpu.device().create_texture(&TextureDescriptor {
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
            }),
            depth: gpu.device().create_texture(&TextureDescriptor {
                label: Some(type_name::<Self>()),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: Surface::DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[Surface::DEPTH_FORMAT],
            }),
        }
    }

    pub fn view(&self) -> FrameView {
        FrameView {
            width: self.color.width(),
            height: self.color.height(),
            color: self.color.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::COLOR_FORMAT),
                ..Default::default()
            }),
            color_srgb: self.color.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::COLOR_SRGB_FORMAT),
                ..Default::default()
            }),
            depth: self.depth.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::DEPTH_FORMAT),
                ..Default::default()
            }),
        }
    }
}

pub struct FrameView {
    width: u32,
    height: u32,
    color: TextureView,
    color_srgb: TextureView,
    depth: TextureView,
}

impl FrameView {
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
}

impl<'a> Frame<'a> {
    pub fn new(surface: SurfaceTexture, depth: &'a Texture) -> Self {
        Self { surface, depth }
    }

    pub fn create_view(&self) -> FrameView {
        FrameView {
            width: self.surface.texture.width(),
            height: self.surface.texture.height(),
            color: self.surface.texture.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::COLOR_FORMAT),
                ..Default::default()
            }),
            color_srgb: self.surface.texture.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::COLOR_SRGB_FORMAT),
                ..Default::default()
            }),
            depth: self.depth.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::DEPTH_FORMAT),
                ..Default::default()
            }),
        }
    }

    pub fn present(self) {
        self.surface.present();
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    depth: Texture,
    format: TextureFormat,
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

        let depth = Self::create_depth_texture(gpu, 1, 1);

        let format = Self::choose_format(surface.get_capabilities(gpu.adapter()));

        let mut surface = Self {
            surface,
            depth,
            format,
        };

        surface.resize(gpu, 1, 1);

        surface
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.surface.configure(
            gpu.device(),
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width,
                height,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![Self::COLOR_SRGB_FORMAT],
            },
        );

        self.depth = Self::create_depth_texture(gpu, width, height);
    }

    pub fn get_current_frame(&self) -> Frame {
        let surface = self
            .surface
            .get_current_texture()
            .expect("Could not obtain surface texture");

        Frame::new(surface, &self.depth)
    }

    pub fn get_current_texture(&self) -> SurfaceTexture {
        self.surface
            .get_current_texture()
            .expect("Could not obtain surface texture")
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

    fn choose_format(capabilities: SurfaceCapabilities) -> TextureFormat {
        if capabilities.formats.contains(&Self::COLOR_FORMAT) {
            return Self::COLOR_FORMAT;
        }

        panic!("Surface Format {:?} is not available", Self::COLOR_FORMAT)
    }

    fn create_depth_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
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
}
