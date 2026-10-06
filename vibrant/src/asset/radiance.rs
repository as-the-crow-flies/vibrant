use std::{any::type_name, mem::size_of, ops::Shr};

use glam::UVec3;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::gpu::Gpu;

pub struct GaussianRadianceBuffer {
    vmm: Texture,
    phi: Texture,
    irradiance: Texture,
    em_iterations: Buffer,
    // `cascade.wgsl` occlusion sources: (include_volume, include_lines, _, _).
    // Combined = (1, 1); overlay volume cascade = (1, 0); overlay line cascade = (0, 1).
    cascade_opts: Buffer,
    binding: BindGroup,
    binding_mipmap: Vec<BindGroup>,
    probes: Vec<UVec3>,
}

impl GaussianRadianceBuffer {
    pub const FORMAT: TextureFormat = TextureFormat::Rgba16Float;
    pub const LEVELS: u32 = 6;

    // Must match EM_ITERATIONS_MAX in em.wgsl: one bucket per
    // possible iteration count (1..=EM_ITERATIONS_MAX), plus a bucket for 0
    // (unused; cascade.wgsl reserves it for culled probes).
    pub const EM_ITERATIONS_MAX: u32 = 100;
    pub const EM_HISTOGRAM_BUCKETS: u32 = Self::EM_ITERATIONS_MAX + 1;

    // One row per cascade level.
    pub const EM_HISTOGRAM_ROWS: u32 = Self::LEVELS;

    pub fn new(gpu: &Gpu, size: UVec3) -> Self {
        let label = Some(type_name::<Self>());

        let size = size.max(UVec3::splat(2u32.pow(Self::LEVELS)));

        let probes = (0..Self::LEVELS)
            .map(|cascade| size.shr(UVec3::splat(cascade)).max(UVec3::ONE))
            .collect();

        let descriptor = TextureDescriptor {
            label,
            size: Extent3d {
                width: size.x * 4,
                height: size.y * 4,
                depth_or_array_layers: size.z * 2,
            },
            mip_level_count: Self::LEVELS,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let phi = gpu.device().create_texture(&descriptor);
        let vmm = gpu.device().create_texture(&descriptor);

        let irradiance = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: size.z,
            },
            mip_level_count: Self::LEVELS,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let em_iterations = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (Self::EM_HISTOGRAM_ROWS * Self::EM_HISTOGRAM_BUCKETS) as u64
                * size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cascade_opts = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&[1u32, 0u32, 0u32, 0u32]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
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
                    resource: BindingResource::TextureView(&phi.create_view(
                        &TextureViewDescriptor {
                            label,
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&vmm.create_view(
                        &TextureViewDescriptor {
                            label,
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let binding_mipmap = (0..Self::LEVELS)
            .into_iter()
            .map(|level| {
                let parent = (level + 1) % Self::LEVELS;

                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout_mipmap(gpu),
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&phi.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: level,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&vmm.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: level,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Sampler(&sampler),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(&phi.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: parent,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: BindingResource::TextureView(&vmm.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: parent,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: BindingResource::TextureView(&irradiance.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: level,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 6,
                            resource: BindingResource::TextureView(&irradiance.create_view(
                                &TextureViewDescriptor {
                                    label,
                                    base_mip_level: parent,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                        BindGroupEntry {
                            binding: 7,
                            resource: em_iterations.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 8,
                            resource: cascade_opts.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();

        Self {
            phi,
            vmm,
            irradiance,
            em_iterations,
            cascade_opts,
            binding,
            binding_mipmap,
            probes,
        }
    }

    pub fn set_cascade_sources(&self, gpu: &Gpu, include_volume: bool, include_lines: bool) {
        gpu.queue().write_buffer(
            &self.cascade_opts,
            0,
            bytemuck::cast_slice(&[include_volume as u32, include_lines as u32, 0u32, 0u32]),
        );
    }

    pub fn size(&self) -> UVec3 {
        let s = self.gaussian().size();
        UVec3::new(s.width, s.height, s.depth_or_array_layers)
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn binding_mipmap(&self, cascade: usize) -> &BindGroup {
        &self.binding_mipmap[cascade]
    }

    pub fn gaussian(&self) -> &Texture {
        &self.vmm
    }

    pub fn radiance(&self) -> &Texture {
        &self.phi
    }

    pub fn irradiance(&self) -> &Texture {
        &self.irradiance
    }

    pub fn probes(&self, cascade: usize) -> UVec3 {
        self.probes[cascade]
    }

    pub fn em_iterations(&self) -> &Buffer {
        &self.em_iterations
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    pub fn layout_mipmap(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // CASCADE_OPTS
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE,
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

impl Drop for GaussianRadianceBuffer {
    fn drop(&mut self) {
        self.em_iterations.destroy();
        self.phi.destroy();
        self.vmm.destroy();
        self.irradiance.destroy();
        self.cascade_opts.destroy();
    }
}
