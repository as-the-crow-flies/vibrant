use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::{TextureDataOrder, TextureViewDescriptor},
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBindingType, BufferUsages, Extent3d, FilterMode, SamplerBindingType, SamplerBorderColor,
    SamplerDescriptor, ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDimension,
};

use crate::{
    file::{VolumeFile, VolumeType},
    gpu::Gpu,
};

pub struct VolumeSegmentationMaterial {
    pub name: String,
    pub absorption: [f32; 3],
    pub scattering: [f32; 3],
    pub anisotropy: f32,
}

impl VolumeSegmentationMaterial {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            absorption: [0.0; 3],
            scattering: [0.0; 3],
            anisotropy: 0.0,
        }
    }

    fn to_buffer(&self) -> VolumeSegmentationMaterialBuffer {
        VolumeSegmentationMaterialBuffer {
            absorption: [
                (self.absorption[0] * 255.0) as u8,
                (self.absorption[1] * 255.0) as u8,
                (self.absorption[2] * 255.0) as u8,
                255,
            ],
            scattering: [
                (self.scattering[0] * 255.0) as u8,
                (self.scattering[1] * 255.0) as u8,
                (self.scattering[2] * 255.0) as u8,
                255,
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VolumeSegmentationMaterialBuffer {
    absorption: [u8; 4],
    scattering: [u8; 4],
}

pub struct VolumeSegmenationBuffer {
    name: String,
    ty: VolumeType,
    size: Extent3d,
    texture: Texture,
    transform: Buffer,
    settings: Vec<VolumeSegmentationMaterial>,
    settings_buffer: Buffer,
    binding: BindGroup,
}

impl VolumeSegmenationBuffer {
    pub fn new(gpu: &Gpu, file: &VolumeFile) -> Self {
        let label = Some(type_name::<Self>());

        let size = Extent3d {
            width: file.dim()[0] as u32,
            height: file.dim()[1] as u32,
            depth_or_array_layers: file.dim()[2] as u32,
        };

        let texture = gpu.device().create_texture_with_data(
            gpu.queue(),
            &TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: match file.ty() {
                    VolumeType::Uint8 => TextureFormat::R8Uint,
                    VolumeType::Uint16 => TextureFormat::R16Uint,
                    VolumeType::Uint32 => TextureFormat::R32Uint,
                    VolumeType::Int8 => TextureFormat::R8Sint,
                    VolumeType::Int16 => TextureFormat::R16Sint,
                    VolumeType::Int32 => TextureFormat::R32Sint,
                    VolumeType::Float32 => TextureFormat::R32Float,
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            file.data(),
        );

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

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&file.transform()),
            usage: BufferUsages::UNIFORM,
        });

        let settings = file
            .ty()
            .is_integer()
            .then(|| {
                vec![
                    VolumeSegmentationMaterial::new("Nothing"),
                    VolumeSegmentationMaterial::new("White Matter"),
                    VolumeSegmentationMaterial::new("Gray Matter"),
                    VolumeSegmentationMaterial::new("CSF"),
                    VolumeSegmentationMaterial::new("Bone"),
                    VolumeSegmentationMaterial::new("Scalp"),
                    VolumeSegmentationMaterial::new("Eyes"),
                    VolumeSegmentationMaterial::new("Compact Bone"),
                    VolumeSegmentationMaterial::new("Spongy Bone"),
                    VolumeSegmentationMaterial::new("Blood"),
                    VolumeSegmentationMaterial::new("Muscle"),
                    VolumeSegmentationMaterial::new("Cartilage"),
                    VolumeSegmentationMaterial::new("Fat"),
                ]
            })
            .unwrap_or_else(|| vec![VolumeSegmentationMaterial::new("Color")]);

        let settings_buffer: Vec<VolumeSegmentationMaterialBuffer> =
            settings.iter().map(|setting| setting.to_buffer()).collect();

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&settings_buffer),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(
                gpu,
                match file.ty() {
                    VolumeType::Uint8 | VolumeType::Uint16 | VolumeType::Uint32 => {
                        TextureSampleType::Uint
                    }
                    VolumeType::Int8 | VolumeType::Int16 | VolumeType::Int32 => {
                        TextureSampleType::Sint
                    }
                    VolumeType::Float32 => TextureSampleType::Float { filterable: true },
                },
            ),
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
                    resource: transform.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: settings_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            name: file.name().to_owned(),
            ty: file.ty(),
            size,
            texture,
            transform,
            settings,
            settings_buffer,
            binding,
        }
    }

    pub fn size(&self) -> Extent3d {
        self.size
    }

    pub fn layout(gpu: &Gpu, ty: TextureSampleType) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: ty,
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
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> VolumeType {
        self.ty
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn settings(&mut self) -> &mut [VolumeSegmentationMaterial] {
        &mut self.settings
    }

    pub fn update_settings(&self, gpu: &Gpu) {
        let settings_buffer: Vec<VolumeSegmentationMaterialBuffer> = self
            .settings
            .iter()
            .map(|setting| setting.to_buffer())
            .collect();

        gpu.queue().write_buffer(
            &self.settings_buffer,
            0,
            bytemuck::cast_slice(&settings_buffer),
        );
    }
}

impl Drop for VolumeSegmenationBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
        self.transform.destroy();
        self.settings_buffer.destroy();
    }
}
