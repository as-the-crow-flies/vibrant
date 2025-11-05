use std::{any::type_name, iter::zip};

use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use random_color::{options::Luminosity, RandomColor};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::{file::LineFile, gpu::Gpu};

pub struct GlobalLineSettings {
    pub selected: Option<bool>,
    pub visible: Option<bool>,
    pub color_visible: bool,
}

pub struct LineSettings {
    pub name: String,
    pub selected: bool,
    pub visible: bool,
    pub color: [u8; 3],
    pub color_visible: bool,
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct LineSettingsBuffer {
    visible: u32,
    color: [u8; 4],
}

impl LineSettings {
    pub fn to_buffer(&self) -> LineSettingsBuffer {
        let [r, g, b] = self.color;

        LineSettingsBuffer {
            visible: self.visible as u32,
            color: [r, g, b, if self.color_visible { 255 } else { 0 }],
        }
    }
}

pub struct LineBuffer {
    global_settings: GlobalLineSettings,
    settings: Vec<LineSettings>,
    settings_buffer: Buffer,

    vertices: Buffer,
    indices: Buffer,
    length: Buffer,
    offset: Buffer,

    materials: Buffer,

    binding_read: BindGroup,
    binding_write: BindGroup,
}

impl LineBuffer {
    pub fn new(gpu: &Gpu, lines: &[LineFile]) -> Self {
        let label = Some(type_name::<Self>());

        let global_settings = GlobalLineSettings {
            selected: Some(false),
            visible: Some(true),
            color_visible: false,
        };

        let settings: Vec<LineSettings> = lines
            .iter()
            .map(|line| LineSettings {
                name: line.name().to_owned(),
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
                color_visible: false,
            })
            .collect();

        let settings_buffer: Vec<LineSettingsBuffer> =
            settings.iter().map(|setting| setting.to_buffer()).collect();

        let vertices: Vec<Vec4> = lines
            .iter()
            .map(|line| line.vertices())
            .flatten()
            .copied()
            .collect();

        let offsets: Vec<u32> = lines
            .iter()
            .map(|line| line.vertices().len() as u32)
            .scan(0u32, |sum, x| {
                *sum += x;
                Some(*sum - x)
            })
            .collect();

        let indices: Vec<u32> = zip(lines.iter().map(|line| line.indices()), offsets)
            .map(|(indices, offset)| indices.iter().map(move |index| offset + index))
            .flatten()
            .collect();

        let materials: Vec<u32> = lines
            .iter()
            .enumerate()
            .map(|(index, line)| [(index as u32)].repeat(line.vertices().len()))
            .flatten()
            .collect();

        let length = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&(indices.len() as u32)),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });

        let vertices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        let indices = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let materials = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&materials),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        });

        let offset = gpu.device().create_buffer(&BufferDescriptor {
            label,
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
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

            vertices,
            indices,
            length,
            offset,

            materials,

            binding_read,
            binding_write,
        }
    }

    pub fn settings_global(&mut self) -> &mut GlobalLineSettings {
        &mut self.global_settings
    }

    pub fn settings(&mut self) -> &mut [LineSettings] {
        &mut self.settings
    }

    pub fn binding(&self, read_only: bool) -> &BindGroup {
        if read_only {
            &self.binding_read
        } else {
            &self.binding_write
        }
    }

    pub fn clear_total_count(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.length, 0, None);
    }

    pub fn clear_count(&self, cmd: &mut CommandEncoder) {
        cmd.clear_buffer(&self.offset, 0, None);
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
                ],
            })
    }
}

impl Drop for LineBuffer {
    fn drop(&mut self) {
        self.vertices.destroy();
        self.indices.destroy();
        self.length.destroy();
        self.offset.destroy();
        self.settings_buffer.destroy();
        self.materials.destroy();
    }
}
