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
            );

        let volumes_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("SelectionVolumes"),
            // 5 x f32/u32 per entry, max 8 entries
            size: 5 * 4 * 8,
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
        let shape: u32 = match settings.selection_volume {
            SelectionVolume::None => return,
            SelectionVolume::Box => 0,
            SelectionVolume::Sphere => 1,
        };

        let data: Vec<u8> = [
            bytemuck::bytes_of(&shape),
            bytemuck::bytes_of(&settings.selection_scale),
            bytemuck::bytes_of(&settings.selection_offset_x),
            bytemuck::bytes_of(&settings.selection_offset_y),
            bytemuck::bytes_of(&settings.selection_offset_z),
        ]
        .concat();

        gpu.queue().write_buffer(&self.volumes_buffer, 0, &data);
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        line: &LineBuffer,
        environment: &Environment,
        settings: &Settings,
    ) {
        let pipeline: &ComputePipeline = match settings.selection_volume {
            SelectionVolume::None => return,
            _ => &self.selection_pipeline,
        };

        self.selection(cmd, line, environment, pipeline);
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
