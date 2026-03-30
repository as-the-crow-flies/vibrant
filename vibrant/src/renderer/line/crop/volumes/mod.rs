use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferDescriptor,
    BufferUsages, CommandEncoder, ComputePassDescriptor, ComputePipeline, ShaderStages,
};

use crate::{
    asset::line::LineBuffer,
    controller::{selection_volume::SelectionVolume, settings::Settings},
    gpu::Gpu,
    renderer::environment::Environment,
};

pub struct LineSelectionPipeline {
    selection_pipeline: ComputePipeline,
    volumes_buffer: Buffer,
    volumes_binding: BindGroup,
}

impl LineSelectionPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let volumes_layout = Self::volumes_layout(gpu);

        let layout = gpu.pipeline_layout(&[
            &LineBuffer::layout(gpu, false),
            &Environment::layout(gpu),
            &volumes_layout,
        ]);

        let preamble_source = include_str!("shapes/preamble.wgsl")
            .to_string()
            .replace(
                "//DISPATCH-INSERT-MARKER//",
                include_str!("shapes/square.wgsl"),
            )
            .replace(
                "//DISPATCH-INSERT-MARKER//",
                include_str!("shapes/sphere.wgsl"),
            )
            .replace(
                "//DISPATCH-INSERT-MARKER//",
                include_str!("shapes/rectangle.wgsl"),
            );

        let volumes_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("SelectionVolumes"),
            // 9 x f32/u32 per entry (shape, scale, x, y, z, negate, size_x, size_y, size_z), max 8 entries
            size: 9 * 4 * 8,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let volumes_binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some("SelectionVolumes"),
            layout: &volumes_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &volumes_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            selection_pipeline: gpu.compute(
                "Selection Pipeline",
                &layout,
                &gpu.shader(&preamble_source),
            ),
            volumes_buffer,
            volumes_binding,
        }
    }

    pub fn update(&self, gpu: &Gpu, settings: &Settings) {
        let mut data: Vec<u8> = Vec::new();

        for vol in &settings.selection_volumes {
            let shape: u32 = match vol.shape {
                SelectionVolume::None => continue,
                SelectionVolume::Box => 0,
                SelectionVolume::Sphere => 1,
                SelectionVolume::Rectangle => 2,
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

        if !data.is_empty() {
            gpu.queue().write_buffer(&self.volumes_buffer, 0, &data);
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        line: &LineBuffer,
        environment: &Environment,
        settings: &Settings,
    ) {
        let has_volumes = settings
            .selection_volumes
            .iter()
            .any(|v| v.shape != SelectionVolume::None);

        if !has_volumes {
            return;
        }

        self.selection(cmd, line, environment, &self.selection_pipeline);
    }

    fn selection(
        &self,
        cmd: &mut CommandEncoder,
        line: &LineBuffer,
        environment: &Environment,
        pipeline: &ComputePipeline,
    ) {
        line.clear_length(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Selection"),
            ..Default::default()
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, line.binding(false), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(2, &self.volumes_binding, &[]);
        pass.dispatch_workgroups(line.n_lines().div_ceil(32), 1, 1);
    }

    fn volumes_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("SelectionVolumes"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
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
