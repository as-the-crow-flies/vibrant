use std::{any::type_name, mem::replace};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, ColorTargetState, ColorWrites,
    CompareFunction, CompositeAlphaMode, DepthBiasState, DepthStencilState, Extent3d, PresentMode,
    ShaderStages, StencilState, SurfaceConfiguration, SurfaceTarget, SurfaceTexture, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension,
};

use super::gpu::Gpu;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    depth: Texture,
    gbuffer: Texture,
}

impl Surface {
    pub const COLOR_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
    pub const COLOR_SRGB_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth16Unorm;
    pub const GBUFFER_FORMAT: TextureFormat = TextureFormat::Rgba32Float;

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
            gbuffer: Self::create_gbuffer_texture(gpu, width, height),
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
            &mut self.gbuffer,
            Self::create_gbuffer_texture(gpu, width, height),
        )
        .destroy();
    }

    pub fn surface_frame(&self, gpu: &Gpu) -> SurfaceFrame {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Could not optain SurfaceTexture");

        SurfaceFrame {
            frame: Frame::new(gpu, &surface_texture.texture, &self.depth, &self.gbuffer),
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

    pub fn gbuffer_target() -> ColorTargetState {
        ColorTargetState {
            format: Surface::GBUFFER_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn gbuffer_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            })
    }

    fn create_color_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
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

    fn create_gbuffer_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
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
            format: Self::GBUFFER_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::GBUFFER_FORMAT],
        })
    }

    fn configuration(width: u32, height: u32) -> SurfaceConfiguration {
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
    gbuffer: TextureView,
    gbuffer_binding: BindGroup,
}

impl Frame {
    pub fn new(gpu: &Gpu, color: &Texture, depth: &Texture, gbuffer: &Texture) -> Self {
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
            gbuffer: gbuffer.create_view(&TextureViewDescriptor {
                label,
                format: Some(Surface::GBUFFER_FORMAT),
                ..Default::default()
            }),
            gbuffer_binding: gpu.device().create_bind_group(&BindGroupDescriptor {
                label,
                layout: &Surface::gbuffer_layout(gpu),
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gbuffer.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Surface::GBUFFER_FORMAT),
                            ..Default::default()
                        },
                    )),
                }],
            }),
        }
    }

    pub fn test(gpu: &Gpu, width: u32, height: u32) -> Self {
        let color = Surface::create_color_texture(gpu, width, height);
        let depth = Surface::create_depth_texture(gpu, width, height);
        let gbuffer = Surface::create_gbuffer_texture(gpu, width, height);

        Self::new(gpu, &color, &depth, &gbuffer)
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

    pub fn gbuffer(&self) -> &TextureView {
        &self.gbuffer
    }

    pub fn gbuffer_binding(&self) -> &BindGroup {
        &self.gbuffer_binding
    }
}
