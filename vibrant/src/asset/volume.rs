use std::any::type_name;

use bytemuck::bytes_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::{TextureDataOrder, TextureViewDescriptor},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferUsages,
    Extent3d, ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDimension,
};

use crate::{
    file::{VolumeFile, VolumeType},
    gpu::Gpu,
};

pub struct VolumeBuffer {
    name: String,
    texture: Texture,
    transform: Buffer,
    binding: BindGroup,
}

impl VolumeBuffer {
    pub fn new(gpu: &Gpu, file: &VolumeFile) -> Self {
        let label = Some(type_name::<Self>());

        let texture = gpu.device().create_texture_with_data(
            gpu.queue(),
            &TextureDescriptor {
                label,
                size: Extent3d {
                    width: file.dim()[0] as u32,
                    height: file.dim()[1] as u32,
                    depth_or_array_layers: file.dim()[2] as u32,
                },
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

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&file.transform()),
            usage: BufferUsages::UNIFORM,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(
                gpu,
                match file.ty() {
                    VolumeType::Uint8 => TextureSampleType::Uint,
                    VolumeType::Uint16 => TextureSampleType::Uint,
                    VolumeType::Uint32 => TextureSampleType::Uint,
                    VolumeType::Int8 => TextureSampleType::Sint,
                    VolumeType::Int16 => TextureSampleType::Sint,
                    VolumeType::Int32 => TextureSampleType::Sint,
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
                    resource: transform.as_entire_binding(),
                },
            ],
        });

        Self {
            name: file.name().to_owned(),
            texture,
            transform,
            binding,
        }
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }
}

impl Drop for VolumeBuffer {
    fn drop(&mut self) {
        self.texture.destroy();
        self.transform.destroy();
    }
}
