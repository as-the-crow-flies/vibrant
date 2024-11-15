use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, Color, ColorTargetState, ColorWrites, Extent3d, LoadOp,
    Operations, RenderPassColorAttachment, ShaderStages, StoreOp, Texture, TextureDescriptor,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::gpu::Gpu;

pub struct DepthHierarchy {
    texture: Texture,

    view: TextureView,
    binding: BindGroup,
    views: Vec<TextureView>,
    bindings: Vec<BindGroup>,
}

impl DepthHierarchy {
    pub const FORMAT: TextureFormat = TextureFormat::R32Float;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        let label = Some(type_name::<Self>());
        let mips = width.min(height).ilog2().clamp(1, 10);

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[Self::FORMAT],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            mip_level_count: Some(1),
            ..Default::default()
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture.create_view(
                    &TextureViewDescriptor {
                        label,
                        ..Default::default()
                    },
                )),
            }],
        });

        let views: Vec<TextureView> = (1..mips)
            .map(|base_mip_level| {
                texture.create_view(&TextureViewDescriptor {
                    label,
                    base_mip_level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();

        let bindings: Vec<BindGroup> = (0..mips - 1)
            .map(|base_mip_level| {
                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout(gpu),
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture.create_view(
                            &TextureViewDescriptor {
                                label,
                                base_mip_level,
                                mip_level_count: Some(1),
                                ..Default::default()
                            },
                        )),
                    }],
                })
            })
            .collect();

        Self {
            texture,
            view,
            binding,
            views,
            bindings,
        }
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            })
    }

    pub fn target() -> ColorTargetState {
        ColorTargetState {
            format: Self::FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn attachment(&self) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: &self.view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::WHITE),
                store: StoreOp::Store,
            },
        }
    }

    pub fn attachments(&self) -> Vec<RenderPassColorAttachment> {
        self.views
            .iter()
            .map(|view| RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::WHITE),
                    store: StoreOp::Store,
                },
            })
            .collect()
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn bindings(&self) -> &[BindGroup] {
        &self.bindings
    }
}

impl Drop for DepthHierarchy {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
