use std::any::type_name;

use bytemuck::bytes_of;
use glam::{Mat4, UVec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::gpu::Gpu;

pub struct PhysicalVolume {
    absorption: Texture,
    scattering: Texture,
    extinction: Texture,

    absorption_u32: Texture,
    scattering_u32: Texture,
    extinction_u32: Texture,

    gradient: Texture,
    ping: Texture,
    pong: Texture,
    transform: Buffer,
    transform_inverse: Buffer,
    binding_read: BindGroup,
    binding_transfer: BindGroup,
    binding_copy: BindGroup,
    binding_gradient_ping: BindGroup,
    binding_gradient_pong: BindGroup,
}

impl PhysicalVolume {
    pub const FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const FORMAT_READWRITE: TextureFormat = TextureFormat::R32Uint;

    pub fn new(gpu: &Gpu, size: UVec3, transform: Mat4) -> Self {
        let label = Some(type_name::<Self>());

        let size = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: size.z,
        };

        let descriptor = TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let descriptor_u32 = TextureDescriptor {
            format: Self::FORMAT_READWRITE,
            ..descriptor
        };

        let absorption = gpu.device().create_texture(&descriptor);
        let scattering = gpu.device().create_texture(&descriptor);
        let extinction = gpu.device().create_texture(&descriptor);

        let absorption_u32 = gpu.device().create_texture(&descriptor_u32);
        let scattering_u32 = gpu.device().create_texture(&descriptor_u32);
        let extinction_u32 = gpu.device().create_texture(&descriptor_u32);

        let gradient = gpu.device().create_texture(&descriptor);

        let absorption_view = absorption.create_view(&TextureViewDescriptor::default());
        let scattering_view = scattering.create_view(&TextureViewDescriptor::default());
        let extinction_view = extinction.create_view(&TextureViewDescriptor::default());

        let absorption_u32_view = absorption_u32.create_view(&TextureViewDescriptor::default());
        let scattering_u32_view = scattering_u32.create_view(&TextureViewDescriptor::default());
        let extinction_u32_view = extinction_u32.create_view(&TextureViewDescriptor::default());

        let gradient_view = gradient.create_view(&TextureViewDescriptor::default());

        let ping = gpu.device().create_texture(&descriptor);
        let pong = gpu.device().create_texture(&descriptor);
        let ping_view = ping.create_view(&TextureViewDescriptor::default());
        let pong_view = pong.create_view(&TextureViewDescriptor::default());

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let transform_inverse = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&transform.inverse()),
            usage: BufferUsages::UNIFORM,
        });

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
        });

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&absorption_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&scattering_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&extinction_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&gradient_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: transform.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: transform_inverse.as_entire_binding(),
                },
            ],
        });

        let binding_transfer = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_transfer(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&absorption_u32_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&scattering_u32_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&extinction_u32_view),
                },
            ],
        });

        let binding_copy = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_copy(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&absorption_u32_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&scattering_u32_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&extinction_u32_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&absorption_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&scattering_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&extinction_view),
                },
            ],
        });

        let binding_gradient_ping = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout_gradient_ping(gpu),
            entries: &[
                // Source
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&extinction_view),
                },
                // Gradient
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gradient_view),
                },
                // Ping
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&ping_view),
                },
                // Pong
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&pong_view),
                },
            ],
        });

        let binding_gradient_pong = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout_gradient_pong(gpu),
            entries: &[
                // Source
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&extinction_view),
                },
                // Gradient
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gradient_view),
                },
                // Ping
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&ping_view),
                },
                // Pong
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&pong_view),
                },
            ],
        });

        Self {
            absorption,
            scattering,
            extinction,
            absorption_u32,
            scattering_u32,
            extinction_u32,
            gradient,
            transform,
            transform_inverse,
            ping,
            pong,
            binding_read,
            binding_transfer,
            binding_copy,
            binding_gradient_ping,
            binding_gradient_pong,
        }
    }

    pub fn size(&self) -> UVec3 {
        let extend = self.absorption.size();

        UVec3::new(extend.width, extend.height, extend.depth_or_array_layers)
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_transfer(&self) -> &BindGroup {
        &self.binding_transfer
    }

    pub fn binding_copy(&self) -> &BindGroup {
        &self.binding_copy
    }

    pub fn binding_gradient_ping(&self) -> &BindGroup {
        &self.binding_gradient_ping
    }

    pub fn binding_gradient_pong(&self) -> &BindGroup {
        &self.binding_gradient_pong
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        let binding_type_read = BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::D3,
            multisampled: false,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: binding_type_read,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: binding_type_read,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: binding_type_read,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: binding_type_read,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility,
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

    pub fn layout_transfer(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: Self::binding_type_readwrite(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: Self::binding_type_readwrite(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: Self::binding_type_readwrite(),
                        count: None,
                    },
                ],
            })
    }

    pub fn layout_copy(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        let read = BindingType::StorageTexture {
            access: StorageTextureAccess::ReadOnly,
            format: Self::FORMAT_READWRITE,
            view_dimension: TextureViewDimension::D3,
        };

        let write = BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format: Self::FORMAT,
            view_dimension: TextureViewDimension::D3,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: read,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: read,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: read,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: write,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility,
                        ty: write,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility,
                        ty: write,
                        count: None,
                    },
                ],
            })
    }

    pub fn layout_gradient_ping(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_write(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_write(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                ],
            })
    }

    pub fn layout_gradient_pong(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_write(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_read(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: Self::binding_type_write(),
                        count: None,
                    },
                ],
            })
    }

    fn binding_type_read() -> BindingType {
        BindingType::StorageTexture {
            access: StorageTextureAccess::ReadOnly,
            format: Self::FORMAT,
            view_dimension: TextureViewDimension::D3,
        }
    }

    fn binding_type_write() -> BindingType {
        BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format: Self::FORMAT,
            view_dimension: TextureViewDimension::D3,
        }
    }

    fn binding_type_readwrite() -> BindingType {
        BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: Self::FORMAT_READWRITE,
            view_dimension: TextureViewDimension::D3,
        }
    }
}

impl Drop for PhysicalVolume {
    fn drop(&mut self) {
        self.absorption.destroy();
        self.scattering.destroy();
        self.extinction.destroy();
        self.absorption_u32.destroy();
        self.scattering_u32.destroy();
        self.extinction_u32.destroy();
        self.gradient.destroy();
        self.transform.destroy();
        self.transform_inverse.destroy();
        self.ping.destroy();
        self.pong.destroy();
    }
}
