use std::{
    any::type_name,
    array,
    sync::atomic::{AtomicU64, Ordering},
};

use bytemuck::{bytes_of, checked::cast_slice, Pod, Zeroable};
use glam::{Mat4, UVec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureDataOrder,
    *,
};

use crate::{
    asset::{
        colormap::{Colormap, ColormapSelection},
        material::{Material, MaterialNode, MaterialPreset},
    },
    file::VolumeFile,
    gpu::Gpu,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    values: [f32; 256],
}

impl Histogram {
    pub fn from_data(data: &[f32]) -> Self {
        let values = data.iter().fold([0u32; 256], |mut hist, value| {
            hist[(value * 255.0) as usize] += 1;
            hist
        });

        let max = 1.0 / values.iter().copied().skip(1).max().unwrap_or(1) as f32;

        Self {
            values: values.map(|value| value as f32 * max),
        }
    }

    pub fn values(&self) -> &[f32; 256] {
        &self.values
    }
}

/// Stable across the volume's lifetime, unlike its Vec index. Used to give UI
/// widgets (e.g. the transfer function editor) identity that doesn't collide
/// across volumes or shift when volumes are added/removed.
static NEXT_VOLUME_ID: AtomicU64 = AtomicU64::new(0);

fn next_volume_id() -> u64 {
    NEXT_VOLUME_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq)]
pub struct VolumeFractionSettings {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub mask: usize,
    pub inverted: bool,
    pub masked: bool,
    pub use_colormap: bool,
    pub colormap: ColormapSelection,
    pub opacity: f32,
    pub histogram: Histogram,
    pub nodes: Vec<MaterialNode>,
}

impl VolumeFractionSettings {
    fn to_buffer(&self) -> VolumeFractionSettingsBuffer {
        VolumeFractionSettingsBuffer {
            inverted: self.inverted as u32,
            masked: self.masked as u32,
            use_colormap: self.use_colormap as u32,
            colormap: self.colormap as u32,
            opacity: self.opacity,
            _pad: [0.0; 3],
            nodes: array::from_fn(|index| {
                self.nodes
                    .get(index)
                    .map(|node| node.into())
                    .unwrap_or(MaterialNodeBuffer {
                        position: 0.0,
                        absorption: [0.0, 0.0, 0.0],
                        scattering: [0.0, 0.0, 0.0],
                        ior: 0.0,
                    })
            }),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct MaterialNodeBuffer {
    absorption: [f32; 3],
    position: f32,
    scattering: [f32; 3],
    ior: f32,
}

impl MaterialNodeBuffer {
    pub const MAX_NODES: usize = 8;
}

impl From<&MaterialNode> for MaterialNodeBuffer {
    fn from(value: &MaterialNode) -> Self {
        MaterialNodeBuffer {
            absorption: value.material.absorption,
            position: value.position,
            scattering: value.material.scattering,
            ior: value.material.ior,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct VolumeFractionSettingsBuffer {
    inverted: u32,
    masked: u32,
    use_colormap: u32,
    colormap: u32,
    opacity: f32,
    _pad: [f32; 3],
    nodes: [MaterialNodeBuffer; MaterialNodeBuffer::MAX_NODES],
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
            id: next_volume_id(),
            name: file.name().to_string(),
            visible: true,
            mask: 0,
            inverted: false,
            masked: true,
            use_colormap: false,
            colormap: ColormapSelection::Viridis,
            opacity: 1.0,
            histogram: Histogram::from_data(file.data()),
            nodes: vec![
                MaterialNode {
                    id: 0,
                    selected: false,
                    position: 0.0,
                    preset: MaterialPreset::Air,
                    material: MaterialPreset::Air.material().unwrap(),
                },
                MaterialNode {
                    id: 1,
                    selected: true,
                    position: 1.0,
                    preset: MaterialPreset::Custom,
                    material: Material {
                        absorption: [1.0, 1.0, 1.0],
                        scattering: [1.0, 1.0, 1.0],
                        ior: 1.5,
                    },
                },
            ],
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
