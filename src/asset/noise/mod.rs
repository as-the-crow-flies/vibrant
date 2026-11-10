use std::any::type_name;

use wgpu::{
    util::DeviceExt, wgt::TextureDataOrder, Extent3d, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::gpu::Gpu;

/// Tiling spatiotemporal blue noise, one slice per sample, four independent channels (see `blue_noise.py`).
pub struct BlueNoise {
    texture: Texture,
    view: TextureView,
}

impl BlueNoise {
    const SIZE: u32 = 64;
    const DEPTH: u32 = 32;

    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let texture = gpu.device().create_texture_with_data(
            gpu.queue(),
            &TextureDescriptor {
                label,
                size: Extent3d {
                    width: Self::SIZE,
                    height: Self::SIZE,
                    depth_or_array_layers: Self::DEPTH,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            include_bytes!("blue_noise.bin"),
        );

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self { texture, view }
    }

    pub fn view(&self) -> &TextureView {
        &self.view
    }
}

impl Drop for BlueNoise {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
