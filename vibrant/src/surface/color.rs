use std::any::type_name;

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Color,
    ColorTargetState, ColorWrites, Extent3d, FilterMode, LoadOp, Operations,
    RenderPassColorAttachment, SamplerBindingType, SamplerDescriptor, ShaderStages, StoreOp,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct ColorBuffer {
    texture: Texture,
    view: TextureView,
    binding: BindGroup,
}

impl ColorBuffer {
    pub const FORMAT: TextureFormat = TextureFormat::Rgba16Float;
    pub const FORMAT_SDR: TextureFormat = TextureFormat::Bgra8Unorm;

    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        Self::new_with_format(gpu, width, height, Self::FORMAT)
    }

    pub fn new_sdr(gpu: &Gpu, width: u32, height: u32) -> Self {
        Self::new_with_format(gpu, width, height, Self::FORMAT_SDR)
    }

    pub fn new_with_format(gpu: &Gpu, width: u32, height: u32, format: TextureFormat) -> Self {
        let label = Some(type_name::<Self>());

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(format),
            ..Default::default()
        });

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            binding,
        }
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn target() -> ColorTargetState {
        ColorTargetState {
            format: Self::FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn target_with_format(format: TextureFormat) -> ColorTargetState {
        ColorTargetState {
            format,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn attachment<'a>(&'a self) -> RenderPassColorAttachment<'a> {
        RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        }
    }

    pub fn attachment_clear<'a>(&'a self) -> RenderPassColorAttachment<'a> {
        RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        }
    }

    pub fn read_pixels(&self, gpu: &crate::gpu::Gpu) -> Vec<u8> {
        let width = self.texture.width();
        let height = self.texture.height();
        let bytes_per_row = ((width * 4 + 255) / 256) * 256;
        let buffer_size = (bytes_per_row * height) as u64;

        let staging = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ColorBuffer::ReadPixels"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Use a separate encoder just for the copy
        let mut copy_cmd = gpu.cmd();
        copy_cmd.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        gpu.submit(copy_cmd);
        gpu.wait();

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.wait();

        let data = slice.get_mapped_range();

        // Strip 256-byte row padding
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }

        pixels
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for ColorBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
