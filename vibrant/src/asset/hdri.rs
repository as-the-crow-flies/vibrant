use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use glam::UVec2;
use half::f16;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureDataOrder,
    *,
};

use crate::{file::hdri::HdriFile, gpu::Gpu};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HdriBufferSettings {
    pub rotation: f32,
    pub strength: f32,
}

pub struct HdriBuffer {
    name: String,
    settings: HdriBufferSettings,

    texture: Texture,
    settings_buffer: Buffer,
    binding: BindGroup,
}

impl HdriBuffer {
    pub fn new(gpu: &Gpu, name: &str, size: UVec2, data: &[[half::f16; 4]]) -> Self {
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

        let settings = HdriBufferSettings {
            rotation: 0.0,
            strength: 1.0,
        };

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&settings),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
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
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &settings_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            name: name.to_string(),
            settings,
            texture,
            settings_buffer,
            binding,
        }
    }

    pub fn from_file(gpu: &Gpu, file: &HdriFile) -> Self {
        Self::new(gpu, file.name(), file.size(), file.data())
    }

    pub fn white(gpu: &Gpu) -> Self {
        Self::new(gpu, "default", UVec2::ONE, &[[f16::ONE; 4]])
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn settings_mut(&mut self) -> &mut HdriBufferSettings {
        &mut self.settings
    }

    pub fn update_settings(&self, gpu: &Gpu) {
        gpu.queue()
            .write_buffer(&self.settings_buffer, 0, bytes_of(&self.settings));
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

impl Drop for HdriBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
