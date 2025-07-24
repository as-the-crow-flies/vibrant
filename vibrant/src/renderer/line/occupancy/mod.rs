use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline, PipelineLayoutDescriptor};

use crate::{
    asset::{
        line::LineSet,
        texture::{MipTexture3D, R32Float, R32Uint, Rgba8Unorm},
    },
    controller::settings::LineVoxelizationMode,
    gpu::Gpu,
    renderer::{environment::Environment, wgsl},
    surface::{occupancy::OccupancyBuffer, Frame},
};

pub struct LineOccupancyPipeline {
    voxelize_tube: ComputePipeline,
    voxelize_line: ComputePipeline,
    voxelize_box: ComputePipeline,
    copy: ComputePipeline,
    mipmap: ComputePipeline,
}

impl LineOccupancyPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let voxelize_layout = &gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[
                    &OccupancyBuffer::layout_write(gpu),
                    &LineSet::layout(gpu, true),
                    &Environment::layout(gpu),
                ],
                push_constant_ranges: &[],
            });

        let voxelize_shader_source = include_str!("voxelize.wgsl");

        Self {
            voxelize_tube: gpu.compute(
                "Occupancy::Voxelize::Tube",
                voxelize_layout,
                &gpu.shader(&(wgsl::voxelize::TUBE.to_owned() + voxelize_shader_source)),
            ),
            voxelize_line: gpu.compute(
                "Occupancy::Voxelize::Line",
                voxelize_layout,
                &gpu.shader(&(wgsl::voxelize::LINE.to_owned() + voxelize_shader_source)),
            ),
            voxelize_box: gpu.compute(
                "Occupancy::Voxelize::Box",
                voxelize_layout,
                &gpu.shader(&(wgsl::voxelize::BOX.to_owned() + voxelize_shader_source)),
            ),
            copy: gpu.compute(
                "Occupancy::Copy",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &OccupancyBuffer::layout_write(gpu),
                            &MipTexture3D::<R32Float>::layout_write(gpu),
                            &MipTexture3D::<R32Uint>::layout_write(gpu),
                            &MipTexture3D::<Rgba8Unorm>::layout_write(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
            mipmap: gpu.compute(
                "Occupancy::MipMap",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[&MipTexture3D::<R32Float>::layout_mipmap(gpu)],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        setting: LineVoxelizationMode,
        tractogram: &LineSet,
    ) {
        frame.occupancy().clear(cmd);
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occupancy"),
            ..Default::default()
        });

        let n = frame.occupancy().density().resolution().div_ceil(8);

        pass.set_bind_group(0, frame.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, tractogram.binding(true), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);

        pass.set_pipeline(match setting {
            LineVoxelizationMode::Line => &self.voxelize_line,
            LineVoxelizationMode::Box => &self.voxelize_box,
            LineVoxelizationMode::Tube => &self.voxelize_tube,
        });
        pass.dispatch_workgroups(64, 1, 1);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, frame.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, frame.occupancy().density().binding_write(), &[]);
        pass.set_bind_group(2, frame.occupancy().count().binding_write(), &[]);
        pass.set_bind_group(3, frame.occupancy().tangent().binding_write(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = frame.occupancy().density().resolution().div_ceil(8);

        for binding in frame.occupancy().density().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
