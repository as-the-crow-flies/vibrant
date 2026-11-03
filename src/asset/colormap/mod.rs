use std::any::type_name;

use strum::EnumIter;
use wgpu::{
    util::DeviceExt, wgt::TextureDataOrder, Extent3d, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};

use crate::gpu::Gpu;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum ColormapSelection {
    Greys,
    Viridis,
    Plasma,
    Inferno,
    Magma,
    Cividis,
}

pub struct Colormap {
    texture: Texture,
}

impl Colormap {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let data = [
            include_bytes!("Greys.bin").as_slice(),
            include_bytes!("viridis.bin").as_slice(),
            include_bytes!("plasma.bin").as_slice(),
            include_bytes!("inferno.bin").as_slice(),
            include_bytes!("magma.bin").as_slice(),
            include_bytes!("cividis.bin").as_slice(),
        ]
        .concat();

        Self {
            texture: gpu.device().create_texture_with_data(
                gpu.queue(),
                &TextureDescriptor {
                    label,
                    size: Extent3d {
                        width: 256,
                        height: 6,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    usage: TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                TextureDataOrder::LayerMajor,
                &data,
            ),
        }
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

impl Drop for Colormap {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
