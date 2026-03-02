use std::any::type_name;

use glam::UVec2;
use half::f16;
use wgpu::{util::DeviceExt, wgt::TextureDataOrder, *};

use crate::{file::hdri::HdriFile, gpu::Gpu};

pub struct HdriBuffer {
    texture: Texture,
    binding: BindGroup,
}

impl HdriBuffer {
    pub fn new(gpu: &Gpu, size: UVec2, data: &[[half::f16; 4]]) -> Self {
        let label = Some(type_name::<Self>());

        let mip_level_count = size.min_element().ilog2();

        let texture = gpu.device().create_texture_with_data(
            gpu.queue(),
            &TextureDescriptor {
                label,
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::MipMajor,
            bytemuck::cast_slice(data),
        );

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: mip_level_count as f32,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self { texture, binding }
    }

    pub fn from_file(gpu: &Gpu, file: &HdriFile) -> Self {
        Self::new(gpu, file.size(), file.data())
    }

    pub fn white(gpu: &Gpu) -> Self {
        Self::new(gpu, UVec2::ONE, &[[f16::ONE; 4]])
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for HdriBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
