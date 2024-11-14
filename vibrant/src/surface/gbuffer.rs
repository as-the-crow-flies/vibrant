use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, ColorTargetState, ColorWrites,
    Extent3d, LoadOp, Operations, RenderPassColorAttachment, ShaderStages, StoreOp, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct GBuffer {
    position: Texture,
    normal: Texture,
    tangent: Texture,
    position_view: TextureView,
    normal_view: TextureView,
    tangent_view: TextureView,
    binding: BindGroup,
}

impl GBuffer {
    pub const POSITION_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
    pub const NORMAL_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const TANGENT_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let label = Some(type_name::<Self>());

        let position = gpu.device().create_texture(&TextureDescriptor {
            label,
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
        });

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

        let position_view = position.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::POSITION_FORMAT),
            ..Default::default()
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

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&position.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::POSITION_FORMAT),
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&normal.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::NORMAL_FORMAT),
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&tangent.create_view(
                        &TextureViewDescriptor {
                            label,
                            format: Some(Self::TANGENT_FORMAT),
                            ..Default::default()
                        },
                    )),
                },
            ],
        });

        Self {
            position,
            normal,
            tangent,
            position_view,
            normal_view,
            tangent_view,
            binding,
        }
    }

    pub fn attachments<'a>(&'a self) -> Vec<Option<RenderPassColorAttachment<'a>>> {
        vec![
            Some(RenderPassColorAttachment {
                view: &self.position_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            }),
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
                ],
            })
    }

    pub fn targets() -> Vec<Option<ColorTargetState>> {
        vec![
            Some(ColorTargetState {
                format: Self::POSITION_FORMAT,
                blend: None,
                write_mask: ColorWrites::all(),
            }),
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
    }

    pub fn view(&self) -> Vec<&TextureView> {
        vec![&self.position_view, &self.normal_view, &self.tangent_view]
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }
}

impl Drop for GBuffer {
    fn drop(&mut self) {
        self.position.destroy();
        self.normal.destroy();
        self.tangent.destroy();
    }
}
