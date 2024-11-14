use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, ColorTargetState, ColorWrites, Extent3d, ShaderStages,
    Texture, TextureDescriptor, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::gpu::Gpu;

pub struct DepthHierarchy {
    texture: Texture,
    views: Vec<TextureView>,
    bindings: Vec<BindGroup>,
}

impl DepthHierarchy {
    pub const FORMAT: TextureFormat = TextureFormat::R32Float;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let label = Some(type_name::<Self>());
        let mips = width.min(height).max(2).next_power_of_two() - 1;

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

        let views: Vec<TextureView> = (0..mips)
            .map(|base_mip_level| {
                texture.create_view(&TextureViewDescriptor {
                    label,
                    base_mip_level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();

        let bindings: Vec<BindGroup> = views
            .iter()
            .map(|view| {
                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout(gpu),
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(view),
                    }],
                })
            })
            .collect();

        Self {
            texture,
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
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

    pub fn views(&self) -> &[TextureView] {
        &self.views
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
