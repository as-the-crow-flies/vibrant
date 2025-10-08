use std::any::type_name;

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendState, Color, ColorTargetState, ColorWrites, Extent3d, FilterMode, LoadOp, Operations,
    RenderPassColorAttachment, SamplerBindingType, SamplerDescriptor, ShaderStages, StoreOp,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct ColorBuffer {
    texture: Texture,
    view: TextureView,
    view_srgb: TextureView,
    binding: BindGroup,
}

impl ColorBuffer {
    pub const FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
    pub const FORMAT_SRGB: TextureFormat = TextureFormat::Bgra8UnormSrgb;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let label = Some(type_name::<Self>());

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
            view_formats: &[Self::FORMAT, Self::FORMAT_SRGB],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::FORMAT),
            ..Default::default()
        });

        let view_srgb = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(Self::FORMAT_SRGB),
            ..Default::default()
        });

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            view_srgb,
            binding,
        }
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn target() -> ColorTargetState {
        ColorTargetState {
            format: Self::FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn target_srgb() -> ColorTargetState {
        ColorTargetState {
            format: Self::FORMAT_SRGB,
            blend: Some(BlendState {
                color: BlendComponent::REPLACE,
                alpha: BlendComponent::OVER,
            }),
            write_mask: ColorWrites::all(),
        }
    }

    pub fn attachment<'a>(&'a self) -> RenderPassColorAttachment<'a> {
        RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        }
    }

    pub fn attachment_clear<'a>(&'a self) -> RenderPassColorAttachment<'a> {
        RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        }
    }

    pub fn attachment_srgb<'a>(&'a self) -> RenderPassColorAttachment<'a> {
        RenderPassColorAttachment {
            view: &self.view_srgb,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        }
    }

    pub fn attachment_srgb_clear<'a>(&'a self) -> RenderPassColorAttachment<'a> {
        RenderPassColorAttachment {
            view: &self.view_srgb,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        }
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for ColorBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
