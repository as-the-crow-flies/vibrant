use std::any::type_name;

use bytemuck::{bytes_of, Pod, Zeroable};
use glam::{Mat4, UVec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::{
    asset::{
        crop::{CropBuffer, CropSettings},
        texture::{MipTexture3D, R32Float},
        volume_fraction::VolumeFractionBuffer,
        volume_mask::VolumeMaskBuffer,
    },
    gpu::Gpu,
};

/// Fractions classified per sample; matches `classify.wgsl`.
pub const SLOTS: usize = 8;
/// Transfer-function lookup table bins; bin i is fraction i / (BINS - 1).
pub const BINS: u32 = 256;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SlotBuffer {
    inverted: u32,
    masked: u32,
    mask_visible: u32,
    mask_invert: u32,
    mask_sdf: u32,
    mask_offset: f32,
    mask_width: f32,
    _pad: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ClassifyBuffer {
    size: [u32; 3],
    count: u32,
    crop: CropSettings,
    slots: [SlotBuffer; SLOTS],
}

/// The shared medium. The volume tracer classifies the fractions per sample
/// through `lut`; the radiance cascades read `extinction`, baked from the same
/// tables, plus the deposited `line_extinction`.
pub struct PhysicalVolume {
    extinction: Texture,
    extinction_write: TextureView,
    // Neutral line density (mipmapped) written by the deposit pass; folded into
    // the radiance cascades' occlusion and the traced shadow rays.
    line_extinction: MipTexture3D<R32Float>,
    // Per BLOCK³ voxels, (absorption, scattering) bounds of the classified
    // material over every channel: the tracer's majorants.
    majorant: Buffer,
    // Per COARSE³ blocks, non-zero where any of them may hold material.
    coarse: Buffer,
    // Layers 2k, 2k + 1: (absorption, ior), (scattering, density) of slot k.
    lut: TextureView,
    lut_slots: Vec<TextureView>,
    classify: Buffer,
    transform: Buffer,
    sampler: Sampler,
    binding_read: BindGroup,
    binding_mipmap: Vec<BindGroup>,
}

impl PhysicalVolume {
    pub const FORMAT: TextureFormat = TextureFormat::Rgba16Float;
    pub const BLOCK: u32 = 4;
    /// Blocks per coarse cell, per axis.
    pub const COARSE: u32 = 8;

    pub fn new(gpu: &Gpu, size: UVec3, transform: Mat4) -> Self {
        let label = Some(type_name::<Self>());

        let mip_level_count = size.min_element().ilog2().max(1);

        let line_extinction =
            MipTexture3D::<R32Float>::new(gpu, size.x, size.y, size.z, AddressMode::ClampToEdge);

        let blocks = Self::blocks(size);

        let extinction = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: size.z,
            },
            mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let level = |base_mip_level| {
            extinction.create_view(&TextureViewDescriptor {
                label,
                base_mip_level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };

        let extinction_view = extinction.create_view(&TextureViewDescriptor {
            label,
            ..Default::default()
        });
        let extinction_write = level(0);

        let majorant = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 8 * blocks.element_product() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let coarse = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4 * Self::coarse_cells(size).element_product() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let lut_texture = gpu.device().create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: BINS,
                height: 1,
                depth_or_array_layers: 2 * SLOTS as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let lut = lut_texture.create_view(&TextureViewDescriptor {
            label,
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });

        let lut_slots = (0..SLOTS as u32)
            .map(|slot| {
                lut_texture.create_view(&TextureViewDescriptor {
                    label,
                    dimension: Some(TextureViewDimension::D2Array),
                    base_array_layer: 2 * slot,
                    array_layer_count: Some(2),
                    ..Default::default()
                })
            })
            .collect();

        let classify = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: size_of::<ClassifyBuffer>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = gpu.device().create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: (mip_level_count + 1) as f32,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&transform),
            usage: BufferUsages::UNIFORM,
        });

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout_read(gpu),
            entries: &[
                vec![
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&extinction_view),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::Sampler(&sampler),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: transform.as_entire_binding(),
                    },
                ],
                // LINE_EXTINCTION texture (8) + sampler (9).
                line_extinction.binding_entries(8),
            ]
            .concat(),
        });

        let binding_mipmap = (0..mip_level_count - 1)
            .map(|level_index| {
                gpu.device().create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &Self::layout_mipmap(gpu),
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&level(level_index)),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&level(level_index + 1)),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Sampler(&sampler),
                        },
                    ],
                })
            })
            .collect();

        Self {
            extinction,
            extinction_write,
            line_extinction,
            majorant,
            coarse,
            lut,
            lut_slots,
            classify,
            transform,
            sampler,
            binding_read,
            binding_mipmap,
        }
    }

    pub fn size(&self) -> UVec3 {
        let extent = self.extinction.size();

        UVec3::new(extent.width, extent.height, extent.depth_or_array_layers)
    }

    /// Occupancy blocks per axis.
    pub fn blocks(size: UVec3) -> UVec3 {
        (size + Self::BLOCK - 1) / Self::BLOCK
    }

    /// Coarse cells per axis.
    pub fn coarse_cells(size: UVec3) -> UVec3 {
        (Self::blocks(size) + Self::COARSE - 1) / Self::COARSE
    }

    pub fn majorant(&self) -> &Buffer {
        &self.majorant
    }

    pub fn coarse(&self) -> &Buffer {
        &self.coarse
    }

    pub fn lut_slots(&self) -> &[TextureView] {
        &self.lut_slots
    }

    pub fn classify(&self) -> &Buffer {
        &self.classify
    }

    pub fn extinction(&self) -> &Texture {
        &self.extinction
    }

    pub fn extinction_write(&self) -> &TextureView {
        &self.extinction_write
    }

    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    pub fn transform(&self) -> &Buffer {
        &self.transform
    }

    /// The neutral line-density texture: `binding_write()` (mip 0) is the line
    /// deposit target, `bindings_mipmap()` builds its chain.
    pub fn line_extinction(&self) -> &MipTexture3D<R32Float> {
        &self.line_extinction
    }

    pub fn binding_read(&self) -> &BindGroup {
        &self.binding_read
    }

    pub fn binding_mipmap(&self) -> &[BindGroup] {
        &self.binding_mipmap
    }

    /// Writes the per-fraction settings for [`Self::binding_classified`].
    pub fn update_classify(
        &self,
        gpu: &Gpu,
        fractions: &[&VolumeFractionBuffer],
        masks: &[VolumeMaskBuffer],
        crop: &CropBuffer,
    ) {
        let mut settings = ClassifyBuffer {
            size: self.size().to_array(),
            count: fractions.len() as u32,
            crop: *crop.settings(),
            ..Zeroable::zeroed()
        };

        for (slot, fraction) in settings.slots.iter_mut().zip(fractions) {
            let mask = masks[fraction.settings().mask].settings();

            *slot = SlotBuffer {
                inverted: fraction.settings().inverted as u32,
                masked: fraction.settings().masked as u32,
                mask_visible: mask.visible as u32,
                mask_invert: mask.inverted as u32,
                mask_sdf: mask.sdf as u32,
                mask_offset: mask.offset,
                mask_width: mask.width,
                _pad: 0,
            };
        }

        gpu.queue()
            .write_buffer(&self.classify, 0, bytes_of(&settings));
    }

    /// `classify.wgsl`'s group: the visible fractions, their masks, the lookup
    /// tables and the majorant grid.
    pub fn binding_classified(
        &self,
        gpu: &Gpu,
        layout: &BindGroupLayout,
        fractions: &[&VolumeFractionBuffer],
        masks: &[VolumeMaskBuffer],
    ) -> BindGroup {
        // Unused slots are never sampled; any 3D float view fills them.
        let placeholder = masks[0].view();

        let fraction_views = (0..SLOTS).map(|k| fractions.get(k).map_or(placeholder, |f| f.view()));
        let mask_views = (0..SLOTS).map(|k| {
            fractions
                .get(k)
                .map_or(placeholder, |f| masks[f.settings().mask].view())
        });

        let mut entries = vec![
            BindGroupEntry {
                binding: 0,
                resource: self.majorant.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&self.sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: self.transform.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(&self.lut),
            },
            BindGroupEntry {
                binding: 4,
                resource: self.classify.as_entire_binding(),
            },
        ];

        entries.extend(
            fraction_views
                .chain(mask_views)
                .zip(5..)
                .map(|(view, binding)| BindGroupEntry {
                    binding,
                    resource: BindingResource::TextureView(view),
                }),
        );
        entries.push(BindGroupEntry {
            binding: 5 + 2 * SLOTS as u32,
            resource: self.coarse.as_entire_binding(),
        });

        gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout,
            entries: &entries,
        })
    }

    pub fn layout_classified(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        let texture = |binding, view_dimension| BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension,
                multisampled: false,
            },
            count: None,
        };

        let buffer = |binding, ty| BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let mut entries = vec![
            buffer(0, BufferBindingType::Storage { read_only: true }),
            BindGroupLayoutEntry {
                binding: 1,
                visibility,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            buffer(2, BufferBindingType::Uniform),
            texture(3, TextureViewDimension::D2Array),
            buffer(4, BufferBindingType::Uniform),
        ];
        entries.extend((5..5 + 2 * SLOTS as u32).map(|b| texture(b, TextureViewDimension::D3)));
        entries.push(buffer(
            5 + 2 * SLOTS as u32,
            BufferBindingType::Storage { read_only: true },
        ));

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &entries,
            })
    }

    pub fn layout_read(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::FRAGMENT | ShaderStages::COMPUTE;

        let uniform = |binding| BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    vec![
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility,
                            ty: Self::binding_type_sample(),
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 5,
                            visibility,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                        uniform(6),
                    ],
                    // LINE_EXTINCTION texture (8) + sampler (9), gated in
                    // `cascade.wgsl` by its `cascade_opts`.
                    MipTexture3D::<R32Float>::layout_entries(8),
                ]
                .concat(),
            })
    }

    pub fn layout_write(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: Self::binding_type_write(),
                    count: None,
                }],
            })
    }

    pub fn layout_mipmap(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: Self::binding_type_sample(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: Self::binding_type_write(),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    fn binding_type_sample() -> BindingType {
        BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::D3,
            multisampled: false,
        }
    }

    fn binding_type_write() -> BindingType {
        BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format: Self::FORMAT,
            view_dimension: TextureViewDimension::D3,
        }
    }
}

impl Drop for PhysicalVolume {
    fn drop(&mut self) {
        self.extinction.destroy();
        self.transform.destroy();
        self.majorant.destroy();
        self.coarse.destroy();
        self.classify.destroy();
    }
}
