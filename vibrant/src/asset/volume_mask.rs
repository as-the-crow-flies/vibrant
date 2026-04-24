use std::any::type_name;

use bytemuck::{bytes_of, checked::cast_slice, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureDataOrder,
    *,
};

use crate::{file::VolumeFile, gpu::Gpu};

#[derive(Debug, Clone)]
pub struct VolumeMaskSettings {
    pub name: String,
    pub visible: bool,
    pub inverted: bool,
    pub offset: f32,
    pub width: f32,
}

impl VolumeMaskSettings {
    fn to_buffer(&self) -> VolumeMaskSettingsBuffer {
        VolumeMaskSettingsBuffer {
            visible: self.visible as u32,
            invert: self.inverted as u32,
            offset: self.offset,
            width: self.width,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VolumeMaskSettingsBuffer {
    pub visible: u32,
    pub invert: u32,
    pub offset: f32,
    pub width: f32,
}

pub struct VolumeMaskBuffer {
    texture: Texture,

    settings: VolumeMaskSettings,
    settings_buffer: Buffer,

    binding: BindGroup,
}

impl VolumeMaskBuffer {
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
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            cast_slice(file.data()),
        );

        let settings = VolumeMaskSettings {
            name: file.name().to_string(),
            visible: true,
            offset: 0.0,
            width: 0.02,
            inverted: false,
        };

        Self::from_texture_settings(gpu, texture, settings)
    }

    pub fn from_texture_settings(
        gpu: &Gpu,
        texture: Texture,
        settings: VolumeMaskSettings,
    ) -> Self {
        let label = Some(type_name::<Self>());

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
                    resource: settings_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            texture,
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

    pub fn settings_mut(&mut self) -> &mut VolumeMaskSettings {
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

    pub fn settings(&self) -> &VolumeMaskSettings {
        &self.settings
    }

    pub fn none(gpu: &Gpu) -> VolumeMaskBuffer {
        let label = Some(type_name::<Self>());

        let texture = gpu.device().create_texture_with_data(
            gpu.queue(),
            &TextureDescriptor {
                label,
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            bytes_of(&1.0),
        );

        let settings = VolumeMaskSettings {
            name: "None".to_string(),
            visible: false,
            offset: 0.0,
            width: 0.00,
            inverted: false,
        };

        Self::from_texture_settings(gpu, texture, settings)
    }
}

impl Drop for VolumeMaskBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
