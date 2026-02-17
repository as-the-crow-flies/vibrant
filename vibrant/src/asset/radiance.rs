use std::any::type_name;

use wgpu::*;

use crate::gpu::Gpu;

pub struct RadianceVolume {
    radiance: Texture,
    cascade_ping: Texture,
    cascade_pong: Texture,
    binding_radiance_read: BindGroup,
    binding_radiance_write: BindGroup,
    binding_ping_read: BindGroup,
    binding_ping_write: BindGroup,
    binding_pong_read: BindGroup,
    binding_pong_write: BindGroup,
}

impl RadianceVolume {
    const FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

    pub fn new(gpu: &Gpu, size: Extent3d) -> Self {
        let label = Some(type_name::<Self>());

        let radiance = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: size.depth_or_array_layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let cascade_descriptor = &TextureDescriptor {
            label,
            size: Extent3d {
                width: size.width.div_ceil(2),
                height: size.height.div_ceil(2),
                depth_or_array_layers: size.depth_or_array_layers.div_ceil(2),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let cascade_ping = gpu.device().create_texture(cascade_descriptor);
        let cascade_pong = gpu.device().create_texture(cascade_descriptor);

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToBorder,
            address_mode_v: AddressMode::ClampToBorder,
            address_mode_w: AddressMode::ClampToBorder,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: Some(SamplerBorderColor::Zero),
        });

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(
                    &radiance.create_view(&TextureViewDescriptor::default()),
                ),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&sampler),
            },
        ];

        let binding_radiance_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries,
        });

        let binding_radiance_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries,
        });

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(
                    &cascade_ping.create_view(&TextureViewDescriptor::default()),
                ),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&sampler),
            },
        ];

        let binding_ping_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries,
        });

        let binding_ping_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries,
        });

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(
                    &cascade_pong.create_view(&TextureViewDescriptor::default()),
                ),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&sampler),
            },
        ];

        let binding_pong_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries,
        });

        let binding_pong_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
            entries,
        });

        Self {
            radiance,
            cascade_ping,
            cascade_pong,
            binding_radiance_read,
            binding_radiance_write,
            binding_ping_read,
            binding_ping_write,
            binding_pong_read,
            binding_pong_write,
        }
    }

    pub fn binding_radiance_read(&self) -> &BindGroup {
        &self.binding_radiance_read
    }

    pub fn binding_radiance_write(&self) -> &BindGroup {
        &self.binding_radiance_write
    }

    pub fn binding_ping_read(&self) -> &BindGroup {
        &self.binding_ping_read
    }

    pub fn binding_ping_write(&self) -> &BindGroup {
        &self.binding_ping_write
    }

    pub fn binding_pong_read(&self) -> &BindGroup {
        &self.binding_pong_read
    }

    pub fn binding_pong_write(&self) -> &BindGroup {
        &self.binding_pong_write
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
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

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
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

impl Drop for RadianceVolume {
    fn drop(&mut self) {
        self.radiance.destroy();
        self.cascade_ping.destroy();
        self.cascade_pong.destroy();
    }
}
