mod importance;

use std::{any::type_name, cell::OnceCell};

use bytemuck::{bytes_of, Pod, Zeroable};
use glam::UVec2;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::{
    file::{hdri::HdriFile, File},
    gpu::Gpu,
};

use importance::{cell_buffer, Cut, Grid, Importance, Lobes};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HdriBufferSettings {
    pub rotation: f32,
    pub strength: f32,
    pub specular: f32,
    pub phase: f32,
    pub roughness: f32,
    pub anisotropy: f32,
    importance: u32,
    prefilter: u32,
    log_weights: u32,
    lookup_jitter: f32,
    lobe_sampling: u32,
    nearest_probes: u32,
    /// Ray traced: uniform share of the environment samples.
    pub traced_uniform_share: f32,
    /// Ray traced: 0 = MIS, 1 = environment only, 2 = BSDF only.
    pub traced_strategy: u32,
    _pad: [u32; 2],
}

pub struct HdriTexture {
    name: String,
    /// Kept to re-cut when the settings change.
    grid: Grid,
    lobes: Lobes,
    cells: Buffer,
    /// The grid's levels: rgb at unit strength, a = luminance. The ray traced
    /// mode reads radiance and warps its samples through it.
    map: TextureView,
    binding: BindGroup,
}

impl HdriTexture {
    pub fn name(&self) -> &str {
        &self.name
    }

    fn new(gpu: &Gpu, name: &str, size: UVec2, data: &[[f32; 3]], settings: &Buffer) -> Self {
        let label = Some(name);

        let grid = Grid::new(size, data);
        let lobes = Lobes::new(gpu);
        let cells = cell_buffer(gpu, "Hdri::Cells");
        let map = environment_map(gpu, &grid);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &HdriBuffer::layout(gpu),
            entries: &[
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: settings,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: lobes.vmm.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: lobes.phi.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: cells.as_entire_binding(),
                },
            ],
        });

        Self {
            name: name.to_string(),
            grid,
            lobes,
            cells,
            map,
            binding,
        }
    }

    fn default(gpu: &Gpu, settings: &Buffer) -> Self {
        Self::new(gpu, "None", UVec2::ONE, &[[1.0; 3]], settings)
    }

    fn from_exr_bytes(gpu: &Gpu, name: &str, bytes: Vec<u8>, settings: &Buffer) -> Self {
        let file = HdriFile::from_exr(&File::new(name, bytes));

        Self::new(gpu, file.name(), file.size(), file.data(), settings)
    }

    fn bake(&self, gpu: &Gpu, importance: &Importance, baked: Baked) {
        importance.bake(
            gpu,
            &self.grid,
            &self.cells,
            &self.lobes,
            baked.cut,
            baked.vmm_size,
        );
    }
}

/// Probe lookup jitter in probe cells (tent, ±2): averaged by accumulation, it
/// blurs away the bands where surfaces cut between probes inside and outside
/// the material.
pub const LOOKUP_JITTER: f32 = 2.0;

pub struct HdriBuffer {
    pub index: usize,
    /// Draw the cascade's samples from the environment's cells instead of uniformly.
    pub importance_sampling: bool,
    /// Blur the parent's lobes by each cascade sample's footprint.
    pub prefilter: bool,
    /// While accumulating, evaluate one sampled lobe per step instead of all.
    pub sample_lobe: bool,
    /// Sampled lobes come from the nearest probe instead of a trilinear blend.
    pub nearest_probes: bool,
    /// Share of the cells spread evenly over the sphere.
    pub uniform_share: f32,
    /// Cut cells by log(1 + Y) instead of Y.
    pub log_sample_weights: bool,
    /// Weight EM samples by log(1 + Y) instead of Y: the fits and the seed lobes.
    pub log_weights: bool,
    baked: Option<Baked>,
    hdris: Vec<Hdri>,
    gpu: Gpu,
    settings: HdriBufferSettings,
    settings_buffer: Buffer,
    importance: Importance,
}

/// A built-in map is decoded and baked on first use; imported maps start loaded.
struct Hdri {
    name: String,
    exr: Option<&'static [u8]>,
    texture: OnceCell<HdriTexture>,
}

impl Hdri {
    fn builtin(name: &str, exr: &'static [u8]) -> Self {
        Self {
            name: name.to_owned(),
            exr: Some(exr),
            texture: OnceCell::new(),
        }
    }

    fn loaded(texture: HdriTexture) -> Self {
        Self {
            name: texture.name().to_owned(),
            exr: None,
            texture: OnceCell::from(texture),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Baked {
    cut: Cut,
    vmm_size: u32,
}

impl HdriBuffer {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let importance = Importance::new(gpu);

        let settings = HdriBufferSettings {
            rotation: 0.0,
            strength: 2.0,
            specular: 1.0,
            phase: 1.0,
            roughness: 0.5,
            anisotropy: 0.0,
            importance: 1,
            prefilter: 1,
            log_weights: 0,
            lookup_jitter: 0.0,
            lobe_sampling: 0,
            nearest_probes: 1,
            traced_uniform_share: 0.1,
            traced_strategy: 0,
            _pad: [0; 2],
        };

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytes_of(&settings),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let hdris = vec![
            Hdri::loaded(HdriTexture::default(gpu, &settings_buffer)),
            Hdri::builtin("Brown", include_bytes!("exr/brown.exr")),
            Hdri::builtin("Christmas", include_bytes!("exr/christmas.exr")),
            Hdri::builtin("Ferndale", include_bytes!("exr/ferndale.exr")),
            Hdri::builtin("Hangar", include_bytes!("exr/hangar.exr")),
            Hdri::builtin("Loft", include_bytes!("exr/loft.exr")),
            Hdri::builtin("Softbox", include_bytes!("exr/softbox.exr")),
            Hdri::builtin("Studio", include_bytes!("exr/studio.exr")),
            Hdri::builtin("Workshop", include_bytes!("exr/workshop.exr")),
        ];

        Self {
            hdris,
            gpu: gpu.clone(),
            index: 0,
            importance_sampling: true,
            prefilter: true,
            sample_lobe: false,
            nearest_probes: true,
            uniform_share: 0.0,
            log_sample_weights: false,
            log_weights: false,
            baked: None,
            settings,
            settings_buffer,
            importance,
        }
    }

    pub fn import(&mut self, gpu: &Gpu, file: HdriFile) {
        let texture = HdriTexture::new(
            gpu,
            file.name(),
            file.size(),
            file.data(),
            &self.settings_buffer,
        );

        if let Some(baked) = self.baked {
            texture.bake(gpu, &self.importance, baked);
        }

        self.hdris.push(Hdri::loaded(texture));
        self.index = self.hdris.len() - 1;
    }

    /// The selected map, decoding (and baking) it on first use.
    pub fn texture(&self) -> &HdriTexture {
        let hdri = &self.hdris[self.index];

        hdri.texture.get_or_init(|| {
            let exr = hdri.exr.expect("unloaded HDRI without EXR data");
            let texture = HdriTexture::from_exr_bytes(
                &self.gpu,
                &hdri.name,
                exr.to_vec(),
                &self.settings_buffer,
            );

            if let Some(baked) = self.baked {
                texture.bake(&self.gpu, &self.importance, baked);
            }

            texture
        })
    }

    pub fn name(&self) -> &str {
        &self.hdris[self.index].name
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.hdris.iter().map(|hdri| hdri.name.as_str())
    }

    pub fn settings_mut(&mut self) -> &mut HdriBufferSettings {
        &mut self.settings
    }

    /// The lookup jitter and lobe sampling only converge while frames accumulate.
    pub fn update_settings(&mut self, gpu: &Gpu, accumulate: bool, jitter: bool) {
        self.settings.importance = self.importance_sampling as u32;
        self.settings.prefilter = self.prefilter as u32;
        self.settings.log_weights = self.log_weights as u32;
        self.settings.lookup_jitter = if accumulate && jitter {
            LOOKUP_JITTER
        } else {
            0.0
        };
        self.settings.lobe_sampling = (accumulate && self.sample_lobe) as u32;
        self.settings.nearest_probes = self.nearest_probes as u32;

        gpu.queue()
            .write_buffer(&self.settings_buffer, 0, bytes_of(&self.settings));
    }

    /// (Re)bakes every loaded texture's cells and VMM when the cut or lobe count
    /// differs from the last bake. Only the radiance cascade reads them, so this
    /// waits until one exists.
    pub fn bake(&mut self, gpu: &Gpu, vmm_size: u32) {
        let baked = Baked {
            cut: Cut {
                log_samples: self.log_sample_weights,
                uniform_share: self.uniform_share,
                log_em: self.log_weights,
            },
            vmm_size,
        };

        if self.baked != Some(baked) {
            for texture in self.hdris.iter().filter_map(|hdri| hdri.texture.get()) {
                texture.bake(gpu, &self.importance, baked);
            }
            self.baked = Some(baked);
        }
    }

    pub fn binding(&self) -> &BindGroup {
        &self.texture().binding
    }

    pub fn map(&self) -> &TextureView {
        &self.texture().map
    }

    pub fn settings_buffer(&self) -> &Buffer {
        &self.settings_buffer
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
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
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
    }
}

impl Drop for HdriTexture {
    fn drop(&mut self) {
        self.lobes.vmm.destroy();
        self.lobes.phi.destroy();
        self.cells.destroy();
    }
}

impl Drop for HdriBuffer {
    fn drop(&mut self) {
        self.settings_buffer.destroy();
    }
}

fn environment_map(gpu: &Gpu, grid: &Grid) -> TextureView {
    let levels = grid.texels().collect::<Vec<_>>();

    let texture = gpu.device().create_texture(&TextureDescriptor {
        label: Some("Hdri::Map"),
        size: Extent3d {
            width: Grid::SIZE,
            height: Grid::SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba32Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    for (level, texels) in levels.iter().enumerate() {
        let size = Grid::SIZE >> level;

        gpu.queue().write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            bytemuck::cast_slice(texels),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(16 * size),
                rows_per_image: Some(size),
            },
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );
    }

    texture.create_view(&TextureViewDescriptor::default())
}
