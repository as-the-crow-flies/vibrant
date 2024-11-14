use std::any::type_name;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt, TextureDataOrder},
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferUsages, Extent3d, FilterMode, SamplerBindingType,
    SamplerDescriptor, ShaderStages, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::{gpu::Gpu, loader::Nifti};

pub struct Volume {
    value: Texture,
    value_view: TextureView,
    color: Texture,
    color_view: TextureView,
    transform: Buffer,
    transform_inverse: Buffer,
    binding: BindGroup,
}

impl Volume {
    pub const VALUE_FORMAT: TextureFormat = TextureFormat::R32Float;
    pub const COLOR_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

    pub fn new(gpu: &Gpu, nifti: &Nifti) -> Self {
        let label = Some(type_name::<Self>());
        let size = Extent3d {
            width: nifti.width(),
            height: nifti.height(),
            depth_or_array_layers: nifti.depth(),
        };

        let value = gpu.device().create_texture_with_data(
            &gpu.queue(),
            &TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: Self::VALUE_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[Self::VALUE_FORMAT],
            },
            TextureDataOrder::default(),
            nifti.data(),
        );

        let value_view = value.create_view(&TextureViewDescriptor {
            label: Some(type_name::<Self>()),
            format: Some(Self::VALUE_FORMAT),
            dimension: Some(TextureViewDimension::D3),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let color = gpu.device().create_texture(&TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::COLOR_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[Self::COLOR_FORMAT],
        });

        let color_view = color.create_view(&TextureViewDescriptor {
            label: Some(type_name::<Self>()),
            format: Some(Self::COLOR_FORMAT),
            dimension: Some(TextureViewDimension::D3),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
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

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&nifti.transform()),
            usage: BufferUsages::UNIFORM,
        });

        let transform_inverse = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&nifti.transform().inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_fragment(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&value_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&color_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &transform_inverse,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            value,
            value_view,
            color,
            color_view,
            transform,
            transform_inverse,
            binding,
        }
    }

    pub fn value(&self) -> &TextureView {
        &self.value_view
    }

    pub fn color(&self) -> &TextureView {
        &self.color_view
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout_fragment(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for Volume {
    fn drop(&mut self) {
        self.value.destroy();
        self.color.destroy();
        self.transform.destroy();
        self.transform_inverse.destroy();
    }
}
