use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use glam::UVec2;
use half::f16;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    wgt::TextureDataOrder,
    *,
};

use crate::{file::hdri::HdriFile, gpu::Gpu};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HdriBufferSettings {
    pub rotation: f32,
    pub strength: f32,
    pub show: u32,
}

pub struct HdriBuffer {
    name: String,
    settings: HdriBufferSettings,

    texture: Texture,
    settings_buffer: Buffer,
    binding: BindGroup,
}

impl HdriBuffer {
    const FORMAT: TextureFormat = TextureFormat::Rgba16Float;

    pub fn new(gpu: &Gpu, name: &str, size: UVec2, data: &[[half::f16; 4]]) -> Self {
        let label = Some(type_name::<Self>());

        let descriptor = TextureDescriptor {
            label,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };

        let staging = gpu.device().create_texture_with_data(
            gpu.queue(),
            &descriptor,
            TextureDataOrder::MipMajor,
            bytemuck::cast_slice(data),
        );

        let texture = gpu.device().create_texture(&descriptor);

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::Repeat,
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

        Self::prefilter(gpu, &staging, &sampler, &texture);

        let settings = HdriBufferSettings {
            rotation: 0.0,
            strength: 1.0,
            show: 0,
        };

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&settings),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
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
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &settings_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            name: name.to_string(),
            settings,
            texture,
            settings_buffer,
            binding,
        }
    }

    pub fn from_file(gpu: &Gpu, file: &HdriFile) -> Self {
        Self::new(gpu, file.name(), file.size(), file.data())
    }

    pub fn white(gpu: &Gpu) -> Self {
        Self::new(gpu, "default", UVec2::ONE, &[[f16::ONE; 4]])
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn settings_mut(&mut self) -> &mut HdriBufferSettings {
        &mut self.settings
    }

    pub fn update_settings(&self, gpu: &Gpu) {
        gpu.queue()
            .write_buffer(&self.settings_buffer, 0, bytes_of(&self.settings));
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
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
                ],
            })
    }

    fn prefilter(gpu: &Gpu, source: &Texture, sampler: &Sampler, destination: &Texture) {
        let source_layout = gpu
            .device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let destination_layout =
            gpu.device()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some(type_name::<Self>()),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: Self::FORMAT,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    }],
                });

        let source_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &source_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &source.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let destination_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &destination_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(
                    &destination.create_view(&TextureViewDescriptor::default()),
                ),
            }],
        });

        let pipeline = gpu.compute(
            type_name::<Self>(),
            &gpu.pipeline_layout(&[&source_layout, &destination_layout]),
            &gpu.shader(include_str!("prefilter.wgsl")),
        );

        let mut cmd = gpu
            .device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &source_binding, &[]);
            pass.set_bind_group(1, &destination_binding, &[]);
            pass.dispatch_workgroups(source.width().div_ceil(8), source.height().div_ceil(8), 1);
        }

        gpu.submit(cmd);
    }
}

impl Drop for HdriBuffer {
    fn drop(&mut self) {
        self.settings_buffer.destroy();
    }
}
