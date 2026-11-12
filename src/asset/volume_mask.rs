use std::any::type_name;

use bytemuck::{bytes_of, checked::cast_slice};
use wgpu::{util::DeviceExt, wgt::TextureDataOrder, *};

use crate::{file::VolumeFile, gpu::Gpu};

#[derive(Debug, Clone)]
pub struct VolumeMaskSettings {
    pub name: String,
    pub visible: bool,
    pub sdf: bool,
    pub inverted: bool,
    pub offset: f32,
    pub width: f32,
}

pub struct VolumeMaskBuffer {
    texture: Texture,
    view: TextureView,

    settings: VolumeMaskSettings,
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
                format: TextureFormat::R16Float,
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
            sdf: file.is_signed(),
        };

        Self::from_texture_settings(texture, settings)
    }

    pub fn size(&self) -> Extent3d {
        self.texture.size()
    }

    pub fn view(&self) -> &TextureView {
        &self.view
    }

    pub fn settings_mut(&mut self) -> &mut VolumeMaskSettings {
        &mut self.settings
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
                format: TextureFormat::R16Float,
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
            sdf: false,
        };

        Self::from_texture_settings(texture, settings)
    }

    fn from_texture_settings(texture: Texture, settings: VolumeMaskSettings) -> Self {
        let view = texture.create_view(&TextureViewDescriptor::default());

        Self {
            texture,
            view,
            settings,
        }
    }
}

impl Drop for VolumeMaskBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
