use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
    BlendOperation, BlendState, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Color,
    ColorTargetState, ColorWrites, Extent3d, LoadOp, Operations, RenderPassColorAttachment,
    ShaderStages, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::gpu::Gpu;

pub struct KBuffer {
    kbuffer: Buffer,
    lock: Buffer,
    binding: BindGroup,
    resolve: Texture,
    resolve_view: TextureView,
    binding_resolve: BindGroup,
}

impl KBuffer {
    pub const RESOLVE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

    pub fn new(gpu: &Gpu, width: u32, height: u32, k: u32) -> Self {
        let label = Some(type_name::<Self>());

        let pixel_count = width.next_multiple_of(8) * height.next_multiple_of(8);

        let kbuffer = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (pixel_count * k * 8) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let lock = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (pixel_count * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: kbuffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: lock.as_entire_binding(),
                },
            ],
        });

        let resolve = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::RESOLVE_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let resolve_view = resolve.create_view(&TextureViewDescriptor::default());

        let binding_resolve = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_resolve(gpu),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&resolve_view),
            }],
        });

        Self {
            kbuffer,
            lock,
            binding,
            resolve,
            resolve_view,
            binding_resolve,
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn binding_resolve(&self) -> &BindGroup {
        &self.binding_resolve
    }

    pub fn resolve(&self) -> &Texture {
        &self.resolve
    }

    pub fn attachment(&self) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: &self.resolve_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        }
    }

    pub fn attachment_clear(&self) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: &self.resolve_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        }
    }

    pub fn target() -> ColorTargetState {
        ColorTargetState {
            format: Self::RESOLVE_FORMAT,
            blend: Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::OneMinusDstAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::OneMinusDstAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            write_mask: ColorWrites::all(),
        }
    }

    pub fn layout_resolve(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            })
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn clear(&self, cmd: &mut wgpu::CommandEncoder) {
        cmd.clear_buffer(&self.kbuffer, 0, None);
        cmd.clear_buffer(&self.lock, 0, None);
    }
}

impl Drop for KBuffer {
    fn drop(&mut self) {
        self.kbuffer.destroy();
        self.resolve.destroy();
    }
}
