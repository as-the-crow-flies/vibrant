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
    gradient: Texture,
    tmp: Texture,
    transform: Buffer,
    sampler: Sampler,
    binding_read: BindGroup,
    binding_write: BindGroup,
    binding_gradient: BindGroup,

    binding_absorption: BindGroup,
    binding_scattering: BindGroup,
    binding_extinction: BindGroup,
    binding_tmp: BindGroup,
}

impl PhysicalVolume {
    pub const FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

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

        let absorption = gpu.device().create_texture(&descriptor);
        let scattering = gpu.device().create_texture(&descriptor);
        let extinction = gpu.device().create_texture(&descriptor);
        let gradient = gpu.device().create_texture(&descriptor);
        let tmp = gpu.device().create_texture(&descriptor);

        let absorption_view = absorption.create_view(&TextureViewDescriptor::default());
        let scattering_view = scattering.create_view(&TextureViewDescriptor::default());
        let extinction_view = extinction.create_view(&TextureViewDescriptor::default());
        let gradient_view = gradient.create_view(&TextureViewDescriptor::default());
        let tmp_view = gradient.create_view(&TextureViewDescriptor::default());

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
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
            ],
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_write(gpu),
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
            ],
        });

        let binding_gradient = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &&Self::layout_gradient(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&extinction_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gradient_view),
                },
            ],
        });

        let binding_absorption = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_texture(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&absorption_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let binding_scattering = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_texture(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&scattering_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let binding_extinction = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_texture(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&extinction_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let binding_tmp = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_texture(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&tmp_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            absorption,
            scattering,
            extinction,
            gradient,
            transform,
            tmp,
            sampler,
            binding_read,
            binding_write,
            binding_gradient,
            binding_absorption,
            binding_scattering,
            binding_extinction,
            binding_tmp,
        }
    }

    pub fn size(&self) -> UVec3 {
        let extend = self.absorption.size();

        UVec3::new(extend.width, extend.height, extend.depth_or_array_layers)
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_write(&self) -> &BindGroup {
        &self.binding_write
    }

    pub fn binding_gradient(&self) -> &BindGroup {
        &self.binding_gradient
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
                ],
            })
    }

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        let binding_type_read_write = BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
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
                        ty: binding_type_read_write,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: binding_type_read_write,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: binding_type_read_write,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: binding_type_read_write,
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
                ],
            })
    }

    pub fn layout_gradient(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: PhysicalVolume::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: PhysicalVolume::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn layout_texture(gpu: &Gpu) -> BindGroupLayout {
        let binding_type = BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: PhysicalVolume::FORMAT,
            view_dimension: TextureViewDimension::D3,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: binding_type,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    pub fn binding_absorption(&self) -> &BindGroup {
        &self.binding_absorption
    }

    pub fn binding_scattering(&self) -> &BindGroup {
        &self.binding_scattering
    }

    pub fn binding_extinction(&self) -> &BindGroup {
        &self.binding_extinction
    }

    pub fn binding_tmp(&self) -> &BindGroup {
        &self.binding_tmp
    }
}

impl Drop for PhysicalVolume {
    fn drop(&mut self) {
        self.absorption.destroy();
        self.scattering.destroy();
        self.extinction.destroy();
        self.gradient.destroy();
        self.transform.destroy();
    }
}
