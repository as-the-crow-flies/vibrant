use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, ColorTargetState, ColorWrites,
    CompareFunction, DepthBiasState, DepthStencilState, Extent3d, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, ShaderStages, StencilState,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct GBuffer {
    normal: Texture,
    tangent: Texture,
    depth: Texture,
    color: Texture,
    normal_view: TextureView,
    tangent_view: TextureView,
    depth_view: TextureView,
    color_view: TextureView,
    binding: BindGroup,
}

impl GBuffer {
    pub const COLOR_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
    pub const NORMAL_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const TANGENT_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let label = Some(type_name::<Self>());

        let normal = gpu.device().create_texture(&TextureDescriptor {
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
        });

        let tangent = gpu.device().create_texture(&TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::TANGENT_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::TANGENT_FORMAT],
        });

        let depth = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::DEPTH_FORMAT],
        });

        let color = gpu.device().create_texture(&TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::COLOR_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::COLOR_FORMAT],
        });

        let normal_view = normal.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::NORMAL_FORMAT),
            ..Default::default()
        });

        let tangent_view = tangent.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::TANGENT_FORMAT),
            ..Default::default()
        });

        let depth_view = depth.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::DEPTH_FORMAT),
            ..Default::default()
        });

        let color_view = color.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::COLOR_FORMAT),
            ..Default::default()
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&normal.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::NORMAL_FORMAT),
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&tangent.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::TANGENT_FORMAT),
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&depth.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::DEPTH_FORMAT),
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&color.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::COLOR_FORMAT),
                            ..Default::default()
                        },
                    )),
                },
            ],
        });

        Self {
            normal,
            tangent,
            depth,
            color,
            normal_view,
            tangent_view,
            depth_view,
            color_view,
            binding,
        }
    }

    pub fn attachment_color<'a>(&'a self) -> Vec<Option<RenderPassColorAttachment<'a>>> {
        [Some(RenderPassColorAttachment {
            view: &self.color_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        })]
        .into()
    }

    pub fn attachment_tangent<'a>(&'a self) -> Vec<Option<RenderPassColorAttachment<'a>>> {
        [Some(RenderPassColorAttachment {
            view: &self.tangent_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        })]
        .into()
    }

    pub fn attachment_normal_tangent<'a>(&'a self) -> Vec<Option<RenderPassColorAttachment<'a>>> {
        vec![
            Some(RenderPassColorAttachment {
                view: &self.normal_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: &self.tangent_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            }),
        ]
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
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
                    BindGroupLayoutEntry {
                        binding: 3,
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

    pub fn target_color() -> Vec<Option<ColorTargetState>> {
        [Some(ColorTargetState {
            format: Self::COLOR_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        })]
        .into()
    }

    pub fn target_tangent() -> Vec<Option<ColorTargetState>> {
        [Some(ColorTargetState {
            format: Self::TANGENT_FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        })]
        .into()
    }

    pub fn target_normal_tangent() -> Vec<Option<ColorTargetState>> {
        [
            Some(ColorTargetState {
                format: Self::NORMAL_FORMAT,
                blend: None,
                write_mask: ColorWrites::all(),
            }),
            Some(ColorTargetState {
                format: Self::TANGENT_FORMAT,
                blend: None,
                write_mask: ColorWrites::all(),
            }),
        ]
        .into()
    }

    pub fn depth_attachment(&self) -> RenderPassDepthStencilAttachment {
        RenderPassDepthStencilAttachment {
            view: &self.depth_view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    pub fn depth_state() -> DepthStencilState {
        DepthStencilState {
            format: Self::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }
}

impl Drop for GBuffer {
    fn drop(&mut self) {
        self.normal.destroy();
        self.tangent.destroy();
        self.depth.destroy();
        self.color.destroy();
    }
}
