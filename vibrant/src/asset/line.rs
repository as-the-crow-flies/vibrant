use std::{any::type_name, iter::zip};

use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use itertools::Itertools;
use random_color::{options::Luminosity, RandomColor};
use strum::EnumIter;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::{
    asset::colormap::{Colormap, ColormapSelection},
    file::{bounds::Bounds, LineFile, TrackScalarFile},
    gpu::Gpu,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum LineColorMode {
    Tangent,
    Color,
    Scalar,
}

#[derive(Debug)]
pub struct GlobalLineSettings {
    pub selected: Option<bool>,
    pub visible: Option<bool>,
    pub color_mode: LineColorMode,
}

#[derive(Debug)]
pub struct LineSettings {
    pub name: String,
    pub offset: u64,
    pub selected: bool,
    pub visible: bool,
    pub color: [u8; 3],
    pub color_mode: LineColorMode,
    pub colormap: ColormapSelection,
    pub crop_start: f32,
    pub crop_end: f32,
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct LineSettingsBuffer {
    visible: u32,
    color: [u8; 4],
    colormap: u32,
    crop_start: f32,
    crop_end: f32,
}

impl LineSettings {
    fn to_buffer(&self) -> LineSettingsBuffer {
        let [r, g, b] = self.color;

        let a = match self.color_mode {
            LineColorMode::Tangent => 0,
            LineColorMode::Color => 128,
            LineColorMode::Scalar => 255,
        };

        LineSettingsBuffer {
            visible: self.visible as u32,
            color: [r, g, b, a],
            colormap: self.colormap as u32,
            crop_start: self.crop_start,
            crop_end: self.crop_end,
        }
    }
}

pub struct LineBuffer {
    global_settings: GlobalLineSettings,
    settings: Vec<LineSettings>,
    settings_buffer: Buffer,

    bounds: Bounds,

    vertices: Buffer,
    indices: Buffer,
    scalar: Buffer,
    length: Buffer,
    offset: Buffer,

    materials: Buffer,
    colormap: Colormap,

    raw_indices: Buffer,
    raw_vertices: Buffer,
    raw_offsets: Buffer,

    transform: Buffer,
    transform_view: Buffer,

    binding_read: BindGroup,
    binding_write: BindGroup,

    n_lines: u32,
}

impl LineBuffer {
    pub fn new(gpu: &Gpu, files: &[LineFile]) -> Self {
        let label = Some(type_name::<Self>());

        let global_settings = GlobalLineSettings {
            selected: Some(false),
            visible: Some(true),
            color_mode: LineColorMode::Tangent,
        };

        let bounds = Bounds::from_bounds(&files.iter().map(|file| *file.bounds()).collect_vec());

        let offsets: Vec<u64> = files
            .iter()
            .map(|file| file.lines().iter().flatten().count() as u64)
            .scan(0u64, |sum, x| {
                *sum += x;
                Some(*sum - x)
            })
            .collect();

        let settings: Vec<LineSettings> = zip(files, offsets)
            .map(|(line, offset)| LineSettings {
                name: line.name().to_owned(),
                offset,
                color: RandomColor {
                    luminosity: Some(Luminosity::Bright),
                    ..Default::default()
                }
                .seed(
                    line.name()
                        .to_lowercase()
                        .replace("_right", "")
                        .replace("_left", "")
                        .replace("_r", "")
                        .replace("_l", "")
                        .replace("right_", "")
                        .replace("left_", "")
                        .replace("l_", "")
                        .replace("r_", ""),
                )
                .clone()
                .into_rgb_array(),
                selected: false,
                visible: true,
                color_mode: LineColorMode::Tangent,
                colormap: ColormapSelection::Greys,
                crop_start: 0.0,
                crop_end: 1.0,
            })
            .collect();

        let settings_buffer: Vec<LineSettingsBuffer> =
            settings.iter().map(|setting| setting.to_buffer()).collect();

        let n_lines = files.iter().map(|file| file.lines().len() as u32).sum();

        let vertices: Vec<Vec4> = files
            .iter()
            .flat_map(|file| file.lines())
            .flatten()
            .copied()
            .collect();

        let vertex_counts: Vec<u32> = files
            .iter()
            .flat_map(|file| file.lines().iter().map(|vertices| vertices.len() as u32))
            .collect();

        let indices: Vec<u32> = vertex_counts
            .iter()
            .scan(0u32, |sum, x| {
                *sum += x;
                Some(*sum - x)
            })
            .tuple_windows()
            .flat_map(|(start, end)| start..end - 1)
            .collect();

        let index_offsets: Vec<u32> = vertex_counts
            .iter()
            .map(|count| count - 1)
            .scan(0u32, |sum, x| {
                *sum += x;
                Some(*sum - x)
            })
            .collect();

        let materials: Vec<u32> = files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                [(index as u32)].repeat(file.lines().iter().map(|line| line.len()).sum())
            })
            .flatten()
            .collect();

        let raw_indices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let raw_vertices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let raw_offsets = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&index_offsets),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let length = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&(indices.len() as u32)),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let vertices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let indices = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: (indices.len() * 8) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let scalar = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: vertices.size() / 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let materials = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&materials),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let colormap = Colormap::new(gpu);

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&bounds.transform().inverse()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let transform_view = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&(Mat4::from_scale(Vec3::ONE * 0.01))),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&settings_buffer),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });

        let entries = [
            BindGroupEntry {
                binding: 0,
                resource: indices.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: vertices.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: length.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: offset.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: materials.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 5,
                resource: settings_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 6,
                resource: raw_indices.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 7,
                resource: raw_vertices.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 8,
                resource: raw_offsets.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 9,
                resource: transform.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 10,
                resource: transform_view.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 11,
                resource: scalar.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 12,
                resource: BindingResource::TextureView(
                    &colormap
                        .texture()
                        .create_view(&TextureViewDescriptor::default()),
                ),
            },
        ];

        let binding_read = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu, true),
            entries: &entries,
        });

        let binding_write = gpu.device().create_bind_group(&BindGroupDescriptor {
            label,
            layout: &Self::layout(gpu, false),
            entries: &entries,
        });

        Self {
            global_settings,
            settings,
            settings_buffer,
            bounds,

            vertices,
            indices,
            scalar,
            length,
            offset,

            materials,
            colormap,

            transform,
            transform_view,

            raw_indices,
            raw_vertices,
            raw_offsets,

            binding_read,
            binding_write,

            n_lines,
        }
    }

    pub fn settings_global(&mut self) -> &mut GlobalLineSettings {
        &mut self.global_settings
    }

    pub fn settings(&mut self) -> &mut [LineSettings] {
        &mut self.settings
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn binding(&self, read_only: bool) -> &BindGroup {
        if read_only {
            &self.binding_read
        } else {
            &self.binding_write
        }
    }

    pub fn clear_offset(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.offset, 0, None);
    }

    pub fn clear_length(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.length, 0, None);
    }

    pub fn update_settings(&self, gpu: &Gpu) {
        let settings_buffer: Vec<LineSettingsBuffer> = self
            .settings
            .iter()
            .map(|setting| setting.to_buffer())
            .collect();

        gpu.queue().write_buffer(
            &self.settings_buffer,
            0,
            bytemuck::cast_slice(&settings_buffer),
        );
    }

    pub fn set_transform(&self, gpu: &Gpu, transform: &Mat4) {
        gpu.queue()
            .write_buffer(&self.transform, 0, bytemuck::bytes_of(transform));

        gpu.queue()
            .write_buffer(&self.transform_view, 0, bytemuck::bytes_of(transform));
    }

    pub fn layout(gpu: &Gpu, read_only: bool) -> BindGroupLayout {
        let visibility = if read_only {
            ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT
        } else {
            ShaderStages::COMPUTE | ShaderStages::FRAGMENT
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Indices
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Vertices
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Length
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Offset
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Materials
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Settings
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Raw Indices
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Raw Vertices
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Raw Offsets
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Transform
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Transform View
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Scalar
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Colormap
                    BindGroupLayoutEntry {
                        binding: 12,
                        visibility,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn n_lines(&self) -> u32 {
        self.n_lines
    }

    pub fn set_scalar(&mut self, gpu: &Gpu, scalar: TrackScalarFile) {
        if let Some(line) = self
            .settings
            .iter_mut()
            .find(|x| scalar.name().contains(&x.name))
        {
            gpu.queue()
                .write_buffer(&self.scalar, line.offset * 4, cast_slice(scalar.values()));

            line.color_mode = LineColorMode::Scalar
        }
    }
}

impl Drop for LineBuffer {
    fn drop(&mut self) {
        self.vertices.destroy();
        self.indices.destroy();
        self.scalar.destroy();
        self.length.destroy();
        self.offset.destroy();
        self.materials.destroy();
        self.settings_buffer.destroy();
        self.raw_indices.destroy();
        self.raw_vertices.destroy();
        self.raw_offsets.destroy();
    }
}
