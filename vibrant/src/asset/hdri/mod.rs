use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use glam::UVec2;
use half::f16;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::{
    file::{hdri::HdriFile, File},
    gpu::Gpu,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HdriBufferSettings {
    pub rotation: f32,
    pub strength: f32,
    pub specular: f32,
    pub roughness: f32,
    pub anisotropy: f32,
}

pub struct HdriTexture {
    name: String,
    texture: Texture,
    binding: BindGroup,
}

impl HdriTexture {
    const FORMAT: TextureFormat = TextureFormat::Rgba16Float;

    pub fn name(&self) -> &str {
        &self.name
    }

    fn new(
        gpu: &Gpu,
        name: &str,
        size: UVec2,
        data: &[[half::f16; 4]],
        sampler: &Sampler,
        settings: &Buffer,
        mipmap: &ComputePipeline,
    ) -> Self {
        let label = Some(name);
        let mip_level_count = size.x.max(size.y).ilog2() + 1;

        let extent = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: extent,
            mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu.queue().write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            bytemuck::cast_slice(data),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.x * 8),
                rows_per_image: Some(size.y),
            },
            extent,
        );

        Self::generate_mipmaps(gpu, &texture, mip_level_count, sampler, mipmap);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &HdriBuffer::layout(gpu),
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
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &settings,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            name: name.to_string(),
            texture,
            binding,
        }
    }

    fn default(gpu: &Gpu, sampler: &Sampler, settings: &Buffer, mipmap: &ComputePipeline) -> Self {
        Self::new(
            gpu,
            "None",
            UVec2::ONE,
            &[[f16::ONE; 4]],
            sampler,
            settings,
            mipmap,
        )
    }

    fn from_exr_bytes(
        gpu: &Gpu,
        name: &str,
        bytes: Vec<u8>,
        sampler: &Sampler,
        settings: &Buffer,
        mipmap: &ComputePipeline,
    ) -> Self {
        let file = HdriFile::from_exr(&File::new(name, bytes));

        Self::new(
            gpu,
            &file.name(),
            file.size(),
            file.data(),
            sampler,
            settings,
            mipmap,
        )
    }

    fn mipmap_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Hdri::Mipmap"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            })
    }

    fn generate_mipmaps(
        gpu: &Gpu,
        texture: &Texture,
        mip_level_count: u32,
        sampler: &Sampler,
        pipeline: &ComputePipeline,
    ) {
        let layout = pipeline.get_bind_group_layout(0);
        let mut cmd = gpu.cmd();
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);

        for level in 1..mip_level_count {
            let size = texture.size().mip_level_size(level, TextureDimension::D2);

            let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture.create_view(
                            &TextureViewDescriptor {
                                base_mip_level: level - 1,
                                mip_level_count: Some(1),
                                ..Default::default()
                            },
                        )),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&texture.create_view(
                            &TextureViewDescriptor {
                                base_mip_level: level,
                                mip_level_count: Some(1),
                                ..Default::default()
                            },
                        )),
                    },
                ],
            });

            pass.set_bind_group(0, &binding, &[]);
            pass.dispatch_workgroups(size.width.div_ceil(8), size.height.div_ceil(8), 1);
        }

        drop(pass);
        gpu.submit(cmd);
    }
}

pub struct HdriBuffer {
    pub index: usize,
    textures: Vec<HdriTexture>,
    sampler: Sampler,
    settings: HdriBufferSettings,
    settings_buffer: Buffer,
    mipmap: ComputePipeline,
}

impl HdriBuffer {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let mipmap = gpu.compute(
            "Hdri::Mipmap",
            &gpu.pipeline_layout(&[&HdriTexture::mipmap_layout(gpu)]),
            &gpu.shader(include_str!("mipmap.wgsl")),
        );

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let settings = HdriBufferSettings {
            rotation: 0.0,
            strength: 1.0,
            specular: 1.0,
            roughness: 0.5,
            anisotropy: 0.0,
        };

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&settings),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let textures = vec![
            HdriTexture::default(gpu, &sampler, &settings_buffer, &mipmap),
            HdriTexture::from_exr_bytes(
                gpu,
                "Brown",
                include_bytes!("brown.exr").to_vec(),
                &sampler,
                &settings_buffer,
                &mipmap,
            ),
            HdriTexture::from_exr_bytes(
                gpu,
                "Country",
                include_bytes!("country.exr").to_vec(),
                &sampler,
                &settings_buffer,
                &mipmap,
            ),
            HdriTexture::from_exr_bytes(
                gpu,
                "Ferndale",
                include_bytes!("ferndale.exr").to_vec(),
                &sampler,
                &settings_buffer,
                &mipmap,
            ),
            HdriTexture::from_exr_bytes(
                gpu,
                "Hangar",
                include_bytes!("hangar.exr").to_vec(),
                &sampler,
                &settings_buffer,
                &mipmap,
            ),
            HdriTexture::from_exr_bytes(
                gpu,
                "Loft",
                include_bytes!("loft.exr").to_vec(),
                &sampler,
                &settings_buffer,
                &mipmap,
            ),
            HdriTexture::from_exr_bytes(
                gpu,
                "Studio",
                include_bytes!("studio.exr").to_vec(),
                &sampler,
                &settings_buffer,
                &mipmap,
            ),
            HdriTexture::from_exr_bytes(
                gpu,
                "Workshop",
                include_bytes!("workshop.exr").to_vec(),
                &sampler,
                &settings_buffer,
                &mipmap,
            ),
        ];

        Self {
            textures,
            sampler,
            index: 0,
            settings,
            settings_buffer,
            mipmap,
        }
    }

    pub fn import(&mut self, gpu: &Gpu, file: HdriFile) {
        self.textures.push(HdriTexture::new(
            gpu,
            file.name(),
            file.size(),
            file.data(),
            &self.sampler,
            &self.settings_buffer,
            &self.mipmap,
        ));
        self.index = self.textures().len() - 1;
    }

    pub fn texture(&self) -> &HdriTexture {
        &self.textures[self.index]
    }

    pub fn textures(&self) -> &[HdriTexture] {
        &self.textures
    }

    pub fn settings_mut(&mut self) -> &mut HdriBufferSettings {
        &mut self.settings
    }

    pub fn update_settings(&self, gpu: &Gpu) {
        gpu.queue()
            .write_buffer(&self.settings_buffer, 0, bytes_of(&self.settings));
    }

    pub fn binding(&self) -> &BindGroup {
        &self.texture().binding
    }

    pub fn settings_buffer(&self) -> &Buffer {
        &self.settings_buffer
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
                    BindGroupLayoutEntry {
                        binding: 2,
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
}

impl Drop for HdriTexture {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}

impl Drop for HdriBuffer {
    fn drop(&mut self) {
        self.settings_buffer.destroy();
    }
}
