use std::{any::type_name, hash::Hash};

use bytemuck::{bytes_of, checked::cast_slice, Pod, Zeroable};
use glam::{Mat4, UVec2, UVec3};
use strum::EnumIter;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureDataOrder,
    *,
};

use crate::{
    asset::colormap::{Colormap, ColormapSelection},
    file::VolumeFile,
    gpu::Gpu,
};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, EnumIter)]
pub enum MaterialPreset {
    Custom,
    White,
    Brain,
    Blood,
}

impl Into<([f32; 3], [f32; 3])> for MaterialPreset {
    fn into(self) -> ([f32; 3], [f32; 3]) {
        match self {
            MaterialPreset::Custom => ([1.0; 3], [1.0; 3]),
            MaterialPreset::White => ([1.0; 3], [1.0; 3]),
            MaterialPreset::Brain => ([0.162, 0.662, 1.0], [1.0, 0.732, 0.575]),
            MaterialPreset::Blood => ([0.510, 0.768, 0.871], [0.544, 0.056, 0.100]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VolumeFractionSettings {
    pub name: String,
    pub visible: bool,
    pub preset: MaterialPreset,
    pub mask: usize,
    pub absorption: [f32; 3],
    pub scattering: [f32; 3],
    pub opacity: f32,
    pub min: f32,
    pub max: f32,
    pub inverted: bool,
    pub masked: bool,
    pub use_colormap: bool,
    pub colormap: ColormapSelection,
}

impl VolumeFractionSettings {
    fn to_buffer(&self) -> VolumeFractionSettingsBuffer {
        let [ar, ag, ab] = self.absorption;
        let [sr, sg, sb] = self.scattering;

        VolumeFractionSettingsBuffer {
            absorption: [ar, ag, ab, self.opacity],
            scattering: [sr, sg, sb, self.opacity],
            min: self.min,
            max: self.max,
            inverted: self.inverted as u32,
            masked: self.masked as u32,
            use_colormap: self.use_colormap as u32,
            colormap: self.colormap as u32,
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct VolumeFractionSettingsBuffer {
    absorption: [f32; 4],
    scattering: [f32; 4],
    min: f32,
    max: f32,
    inverted: u32,
    masked: u32,
    use_colormap: u32,
    colormap: u32,
    padding: UVec2,
}

pub struct VolumeFractionBuffer {
    texture: Texture,
    size: UVec3,

    transform: Mat4,
    transform_buffer: Buffer,

    settings: VolumeFractionSettings,
    settings_buffer: Buffer,

    binding: BindGroup,
}

impl VolumeFractionBuffer {
    pub fn new(gpu: &Gpu, file: &VolumeFile, colormap: &Colormap) -> Self {
        let label = Some(type_name::<Self>());

        let size = Extent3d {
            width: file.size().x,
            height: file.size().y,
            depth_or_array_layers: file.size().z,
        };

        let texture = gpu.device().create_texture_with_data(
            gpu.queue(),
            &TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            cast_slice(file.data()),
        );

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

        let transform = file.transform();

        let transform_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
        });

        let settings = VolumeFractionSettings {
            name: file.name().to_string(),
            visible: true,
            preset: MaterialPreset::White,
            mask: 0,
            absorption: [1.0; 3],
            scattering: [1.0; 3],
            opacity: 1.0,
            min: 0.0,
            max: 1.0,
            inverted: false,
            masked: true,
            use_colormap: false,
            colormap: ColormapSelection::Viridis,
        };

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&settings.to_buffer()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
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
                    resource: transform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: settings_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &colormap
                            .texture()
                            .create_view(&TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        Self {
            texture,
            size: file.size(),
            transform,
            transform_buffer,
            settings,
            settings_buffer,
            binding,
        }
    }

    pub fn size(&self) -> UVec3 {
        self.size
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn transform(&self) -> Mat4 {
        self.transform
    }

    pub fn settings(&self) -> &VolumeFractionSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut VolumeFractionSettings {
        &mut self.settings
    }

    pub fn update_settings(&self, gpu: &Gpu) {
        gpu.queue().write_buffer(
            &self.settings_buffer,
            0,
            bytes_of(&self.settings.to_buffer()),
        );
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Texture
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
                    // Sampler
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Transform
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
                    // Settings
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Colormap
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for VolumeFractionBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
        self.transform_buffer.destroy();
    }
}
