use std::{any::type_name, iter::zip};

use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};
use glam::{Mat4, Vec4};
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
    pub visible: bool,
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

    raw_indices: Buffer,
    raw_vertices: Buffer,
    raw_offsets: Buffer,

    transform: Buffer,

    binding_transform: BindGroup,
    binding_crop: BindGroup,
    binding_render: BindGroup,

    n_lines: u32,
}

impl LineBuffer {
    pub fn new(gpu: &Gpu, files: &[LineFile], colormap: &Colormap) -> Self {
        let label = Some(type_name::<Self>());

        let global_settings = GlobalLineSettings {
            selected: Some(false),
            visible: true,
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

        let transform = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&bounds.transform().inverse()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let settings_buffer = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&settings_buffer),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });

        let binding_transform = gpu.binding(
            "LineTransform",
            &Self::layout_transform(gpu),
            vec![
                indices.as_entire_binding(),
                vertices.as_entire_binding(),
                length.as_entire_binding(),
                offset.as_entire_binding(),
                raw_vertices.as_entire_binding(),
                transform.as_entire_binding(),
            ],
        );

        let binding_crop = gpu.binding(
            "LineCrop",
            &Self::layout_crop(gpu),
            vec![
                indices.as_entire_binding(),
                vertices.as_entire_binding(),
                length.as_entire_binding(),
                materials.as_entire_binding(),
                settings_buffer.as_entire_binding(),
                raw_indices.as_entire_binding(),
                raw_offsets.as_entire_binding(),
            ],
        );

        let binding_render = gpu.binding(
            "LineRender",
            &Self::layout_render(gpu),
            vec![
                indices.as_entire_binding(),
                vertices.as_entire_binding(),
                length.as_entire_binding(),
                offset.as_entire_binding(),
                materials.as_entire_binding(),
                settings_buffer.as_entire_binding(),
                transform.as_entire_binding(),
                scalar.as_entire_binding(),
                BindingResource::TextureView(
                    &colormap
                        .texture()
                        .create_view(&TextureViewDescriptor::default()),
                ),
            ],
        );

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

            transform,

            raw_indices,
            raw_vertices,
            raw_offsets,
            binding_transform,
            binding_crop,
            binding_render,

            n_lines,
        }
    }

    pub fn settings_global_mut(&mut self) -> &mut GlobalLineSettings {
        &mut self.global_settings
    }

    pub fn settings_mut(&mut self) -> &mut [LineSettings] {
        &mut self.settings
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn binding_transform(&self) -> &BindGroup {
        &self.binding_transform
    }

    pub fn binding_crop(&self) -> &BindGroup {
        &self.binding_crop
    }

    pub fn binding_render(&self) -> &BindGroup {
        &self.binding_render
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
            .write_buffer(&self.transform, 0, bytes_of(transform));
    }

    pub fn layout_transform(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE;

        let buffer_readwrite = BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let buffer_read = BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Indices
                    BindGroupLayoutEntry {
                        binding: 0,
                        ..buffer_readwrite
                    },
                    // Vertices
                    BindGroupLayoutEntry {
                        binding: 1,
                        ..buffer_readwrite
                    },
                    // Length
                    BindGroupLayoutEntry {
                        binding: 2,
                        ..buffer_readwrite
                    },
                    // Offset
                    BindGroupLayoutEntry {
                        binding: 3,
                        ..buffer_readwrite
                    },
                    // Raw Vertices
                    BindGroupLayoutEntry {
                        binding: 4,
                        ..buffer_read
                    },
                    // Transform
                    BindGroupLayoutEntry {
                        binding: 5,
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

    pub fn layout_crop(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE;

        let buffer_readwrite = BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let buffer_read = BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Indices
                    BindGroupLayoutEntry {
                        binding: 0,
                        ..buffer_readwrite
                    },
                    // Vertices
                    BindGroupLayoutEntry {
                        binding: 1,
                        ..buffer_readwrite
                    },
                    // Length
                    BindGroupLayoutEntry {
                        binding: 2,
                        ..buffer_readwrite
                    },
                    // Material
                    BindGroupLayoutEntry {
                        binding: 3,
                        ..buffer_read
                    },
                    // Settings
                    BindGroupLayoutEntry {
                        binding: 4,
                        ..buffer_read
                    },
                    // Index Raw
                    BindGroupLayoutEntry {
                        binding: 5,
                        ..buffer_read
                    },
                    // Offset Raw
                    BindGroupLayoutEntry {
                        binding: 6,
                        ..buffer_read
                    },
                ],
            })
    }

    pub fn layout_render(gpu: &Gpu) -> BindGroupLayout {
        let visibility = ShaderStages::COMPUTE | ShaderStages::FRAGMENT;

        let buffer_read = BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let buffer_readwrite = BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    // Indices
                    BindGroupLayoutEntry {
                        binding: 0,
                        ..buffer_read
                    },
                    // Vertices
                    BindGroupLayoutEntry {
                        binding: 1,
                        ..buffer_read
                    },
                    // Length
                    BindGroupLayoutEntry {
                        binding: 2,
                        ..buffer_read
                    },
                    // Offset
                    BindGroupLayoutEntry {
                        binding: 3,
                        ..buffer_readwrite
                    },
                    // Material
                    BindGroupLayoutEntry {
                        binding: 4,
                        ..buffer_read
                    },
                    // Settings
                    BindGroupLayoutEntry {
                        binding: 5,
                        ..buffer_read
                    },
                    // Transform
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Scalar
                    BindGroupLayoutEntry {
                        binding: 7,
                        ..buffer_read
                    },
                    // Colormap
                    BindGroupLayoutEntry {
                        binding: 8,
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
