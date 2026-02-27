use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureDataOrder,
    *,
};

use crate::{file::VolumeFile, gpu::Gpu};

#[derive(Debug, Clone)]
pub struct VolumeFractionSettings {
    pub name: String,
    pub visible: bool,
    pub absorption: [f32; 3],
    pub scattering: [f32; 3],
}

impl VolumeFractionSettings {
    fn to_buffer(&self) -> VolumeFractionSettingsBuffer {
        let [ar, ag, ab] = self.absorption;
        let [sr, sg, sb] = self.scattering;

        VolumeFractionSettingsBuffer {
            absorption: [ar, ag, ab, 0.0],
            scattering: [sr, sg, sb, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VolumeFractionSettingsBuffer {
    absorption: [f32; 4],
    scattering: [f32; 4],
}

pub struct VolumeFractionBuffer {
    texture: Texture,
    transform: Buffer,

    settings: VolumeFractionSettings,
    settings_buffer: Buffer,

    binding: BindGroup,
}

impl VolumeFractionBuffer {
    pub fn new(gpu: &Gpu, file: &VolumeFile) -> Self {
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
                format: file.ty().into(),
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            file.data(),
        );

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
            contents: bytes_of(&file.transform()),
            usage: BufferUsages::UNIFORM,
        });

        let settings = VolumeFractionSettings {
            name: file.name().to_string(),
            visible: true,
            absorption: [0.2; 3],
            scattering: [0.2; 3],
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
                    resource: transform.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: settings_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            texture,
            transform,
            settings,
            settings_buffer,
            binding,
        }
    }

    pub fn size(&self) -> Extent3d {
        self.texture.size()
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
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
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn settings(&self) -> &VolumeFractionSettings {
        &self.settings
    }
}

impl Drop for VolumeFractionBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
        self.transform.destroy();
    }
}
