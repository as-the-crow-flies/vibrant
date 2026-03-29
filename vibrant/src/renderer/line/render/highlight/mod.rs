use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder,
    RenderPassDescriptor, RenderPipeline, ShaderStages,
};

use crate::{
    controller::{selection_volume::SelectionVolume, settings::Settings},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

pub struct VolumeHighlightPipeline {
    pipeline: RenderPipeline,
    highlight_buffer: Buffer,
    highlight_binding: BindGroup,
}

impl VolumeHighlightPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let highlight_layout = Self::highlight_layout(gpu);

        let layout = gpu.pipeline_layout(&[&Environment::layout(gpu), &highlight_layout]);

        let pipeline = gpu.quad(
            "VolumeHighlight",
            &layout,
            &[Some(ColorTargetState {
                format: ColorBuffer::FORMAT,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::all(),
            })],
            &gpu.shader(include_str!("highlight.wgsl")),
        );

        // 1 u32 (count) + 8 entries * 9 u32s each (shape, scale, x, y, z, negate, size_x, size_y, size_z) = 73 u32s = 292 bytes
        let highlight_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("HighlightVolumes"),
            size: 320,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let highlight_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some("HighlightVolumes"),
            layout: &highlight_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &highlight_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            pipeline,
            highlight_buffer,
            highlight_binding,
        }
    }

    pub fn update(&self, gpu: &Gpu, settings: &Settings) {
        let highlighted: Vec<_> = settings
            .selection_volumes
            .iter()
            .filter(|v| v.highlight && v.shape != SelectionVolume::None)
            .collect();

        let count = highlighted.len() as u32;
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(bytemuck::bytes_of(&count));

        for vol in &highlighted {
            let shape: u32 = match vol.shape {
                SelectionVolume::Box => 0,
                SelectionVolume::Sphere => 1,
                SelectionVolume::Rectangle => 2,
                SelectionVolume::None => continue,
            };
            data.extend_from_slice(bytemuck::bytes_of(&shape));
            data.extend_from_slice(bytemuck::bytes_of(&vol.scale));
            data.extend_from_slice(bytemuck::bytes_of(&vol.offset_x));
            data.extend_from_slice(bytemuck::bytes_of(&vol.offset_y));
            data.extend_from_slice(bytemuck::bytes_of(&vol.offset_z));
            let negate: u32 = vol.negate as u32;
            data.extend_from_slice(bytemuck::bytes_of(&negate));
            data.extend_from_slice(bytemuck::bytes_of(&vol.size_x));
            data.extend_from_slice(bytemuck::bytes_of(&vol.size_y));
            data.extend_from_slice(bytemuck::bytes_of(&vol.size_z));
        }

        gpu.queue().write_buffer(&self.highlight_buffer, 0, &data);
    }

    pub fn render(&self, cmd: &mut CommandEncoder, environment: &Environment, frame: &Frame) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some("VolumeHighlight"),
            color_attachments: &[Some(frame.highlight().attachment_clear())],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, &self.highlight_binding, &[]);
        pass.draw(0..4, 0..1);
    }

    fn highlight_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("HighlightVolumes"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}
