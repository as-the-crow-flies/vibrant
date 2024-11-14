use std::{any::type_name, mem::replace};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferDescriptor,
    BufferUsages, Color, ColorTargetState, ColorWrites, CompareFunction, CompositeAlphaMode,
    DepthBiasState, DepthStencilState, Extent3d, LoadOp, Operations, PresentMode,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, ShaderStages, StencilState,
    StoreOp, SurfaceConfiguration, SurfaceTarget, SurfaceTexture, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};

use super::gpu::Gpu;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    visibility: Buffer,
    depth: Texture,
    position: Texture,
    normal: Texture,
    tangent: Texture,
}

impl Surface {
    pub const COLOR_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
    pub const COLOR_SRGB_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth16Unorm;
    pub const DEPTH_HIERARCHY_FORMAT: TextureFormat = TextureFormat::R32Float;
    pub const POSITION_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
    pub const NORMAL_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let (width, height) = (1, 1);

        surface.configure(gpu.device(), &Self::configuration(width, height));

        Self {
            surface,
            visibility: Self::create_visibility_buffer(gpu, width, height),
            depth: Self::create_depth_texture(gpu, width, height),
            position: Self::create_position_texture(gpu, width, height),
            normal: Self::create_normal_texture(gpu, width, height),
            tangent: Self::create_normal_texture(gpu, width, height),
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

        replace(
            &mut self.position,
            Self::create_position_texture(gpu, width, height),
        )
        .destroy();

        replace(
            &mut self.normal,
            Self::create_normal_texture(gpu, width, height),
        )
        .destroy();

        replace(
            &mut self.tangent,
            Self::create_normal_texture(gpu, width, height),
        )
        .destroy();
    }

    pub fn surface_frame(&self, gpu: &Gpu) -> SurfaceFrame {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Could not optain SurfaceTexture");

        SurfaceFrame {
            frame: Frame::new(
                gpu,
                &surface_texture.texture,
                &self.depth,
                &self.visibility,
                &self.position,
                &self.normal,
                &self.tangent,
            ),
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

    pub fn position_target() -> ColorTargetState {
        ColorTargetState {
            format: Surface::POSITION_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn normal_target() -> ColorTargetState {
        ColorTargetState {
            format: Surface::NORMAL_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn tangent_target() -> ColorTargetState {
        ColorTargetState {
            format: Surface::NORMAL_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn visibility(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }

    pub fn gbuffer(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
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

    fn create_visibility_buffer(gpu: &Gpu, width: u32, height: u32) -> Buffer {
        gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: (width * height * 4) as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_position_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
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
            format: Self::POSITION_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::POSITION_FORMAT],
        })
    }

    fn create_normal_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
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
            format: Self::NORMAL_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::NORMAL_FORMAT],
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
    position: TextureView,
    normal: TextureView,
    tangent: TextureView,
    visibility: BindGroup,
    gbuffer: BindGroup,
}

impl Frame {
    pub fn new(
        gpu: &Gpu,
        color: &Texture,
        depth: &Texture,
        visibility: &Buffer,
        position: &Texture,
        normal: &Texture,
        tangent: &Texture,
    ) -> Self {
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
            position: position.create_view(&TextureViewDescriptor {
                label,
                format: Some(Surface::POSITION_FORMAT),
                ..Default::default()
            }),
            normal: normal.create_view(&TextureViewDescriptor {
                label,
                format: Some(Surface::NORMAL_FORMAT),
                ..Default::default()
            }),
            tangent: tangent.create_view(&TextureViewDescriptor {
                label,
                format: Some(Surface::NORMAL_FORMAT),
                ..Default::default()
            }),
            visibility: gpu.device().create_bind_group(&BindGroupDescriptor {
                label,
                layout: &Surface::visibility(gpu),
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &visibility,
                        offset: 0,
                        size: None,
                    }),
                }],
            }),
            gbuffer: gpu.device().create_bind_group(&BindGroupDescriptor {
                label,
                layout: &Surface::gbuffer(gpu),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&position.create_view(
                            &TextureViewDescriptor {
                                label,
                                format: Some(Surface::POSITION_FORMAT),
                                ..Default::default()
                            },
                        )),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&normal.create_view(
                            &TextureViewDescriptor {
                                label,
                                format: Some(Surface::NORMAL_FORMAT),
                                ..Default::default()
                            },
                        )),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&tangent.create_view(
                            &TextureViewDescriptor {
                                label,
                                format: Some(Surface::NORMAL_FORMAT),
                                ..Default::default()
                            },
                        )),
                    },
                ],
            }),
        }
    }

    pub fn test(gpu: &Gpu, width: u32, height: u32) -> Self {
        let color = Surface::create_color_texture(gpu, width, height);
        let depth = Surface::create_depth_texture(gpu, width, height);
        let visibility = Surface::create_visibility_buffer(gpu, width, height);
        let position = Surface::create_position_texture(gpu, width, height);
        let normal = Surface::create_normal_texture(gpu, width, height);
        let tangent = Surface::create_normal_texture(gpu, width, height);

        Self::new(
            gpu,
            &color,
            &depth,
            &visibility,
            &position,
            &normal,
            &tangent,
        )
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

    pub fn position(&self) -> &TextureView {
        &self.position
    }

    pub fn normal(&self) -> &TextureView {
        &self.normal
    }

    pub fn tangent(&self) -> &TextureView {
        &self.tangent
    }

    pub fn visibility(&self) -> &BindGroup {
        &self.visibility
    }

    pub fn gbuffer(&self) -> &BindGroup {
        &self.gbuffer
    }

    pub fn gbuffer_attachment<'a>(&'a self) -> Vec<Option<RenderPassColorAttachment<'a>>> {
        vec![
            Some(RenderPassColorAttachment {
                view: self.position(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: self.normal(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: self.tangent(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            }),
        ]
    }

    pub fn depth_attachment(&self) -> RenderPassDepthStencilAttachment {
        RenderPassDepthStencilAttachment {
            view: self.depth(),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
}
