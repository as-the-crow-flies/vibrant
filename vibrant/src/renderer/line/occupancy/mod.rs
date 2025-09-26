use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::LineSet,
        texture::{MipTexture3D, R32Float, R32Uint},
    },
    controller::settings::{LineVoxelizationMode, Settings},
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
        let voxelize_layout = &gpu.pipeline_layout(&[
            &OccupancyBuffer::layout_write(gpu),
            &LineSet::layout(gpu, true),
            &Environment::layout(gpu),
        ]);

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
                &gpu.pipeline_layout(&[
                    &OccupancyBuffer::layout_write(gpu),
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &MipTexture3D::<R32Uint>::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
            mipmap: gpu.compute(
                "Occupancy::MipMap",
                &gpu.pipeline_layout(&[&MipTexture3D::<R32Float>::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        settings: &Settings,
        line: &LineSet,
    ) {
        frame.occupancy().clear(cmd);
        line.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occupancy"),
            ..Default::default()
        });

        let n = frame.occupancy().pyramid().resolution().div_ceil(8);

        pass.set_bind_group(0, frame.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, line.binding(true), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);

        pass.set_pipeline(match settings.voxelization {
            LineVoxelizationMode::Line => &self.voxelize_line,
            LineVoxelizationMode::Box => &self.voxelize_box,
            LineVoxelizationMode::Tube => &self.voxelize_tube,
        });
        pass.dispatch_workgroups(settings.workgroups, 1, 1);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, frame.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, frame.occupancy().pyramid().binding_write(), &[]);
        pass.set_bind_group(2, frame.occupancy().count().binding_write(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = frame.occupancy().pyramid().resolution().div_ceil(8);

        for binding in frame.occupancy().pyramid().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
