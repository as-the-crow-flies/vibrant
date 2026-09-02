use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::{culling::CullingBuffer, LineBuffer},
        texture::{MipTexture3D, R32Float, R32Uint},
    },
    controller::settings::{LineVoxelizationMode, Settings},
    gpu::Gpu,
    renderer::{environment::Environment, wgsl},
};

pub struct LinePopulatePipeline {
    scan: ComputePipeline,
    populate_tube: ComputePipeline,
    populate_line: ComputePipeline,
    populate_box: ComputePipeline,
}

impl LinePopulatePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = &gpu.pipeline_layout(&[
            &CullingBuffer::layout_write(gpu),
            &MipTexture3D::<R32Float>::layout(gpu),
            &LineBuffer::layout_render(gpu),
            &Environment::layout(gpu),
        ]);

        let populate_source = include_str!("populate.wgsl");

        Self {
            scan: gpu.compute(
                "Populate::Scan",
                &gpu.pipeline_layout(&[
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &CullingBuffer::layout_write(gpu),
                    &MipTexture3D::<R32Uint>::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("scan.wgsl")),
            ),
            populate_tube: gpu.compute(
                "Populate::Populate::Tube",
                layout,
                &gpu.shader(&(wgsl::voxelize::TUBE.to_owned() + populate_source)),
            ),
            populate_line: gpu.compute(
                "Populate::Populate::Line",
                layout,
                &gpu.shader(&(wgsl::voxelize::LINE.to_owned() + populate_source)),
            ),
            populate_box: gpu.compute(
                "Populate::Populate::Box",
                layout,
                &gpu.shader(&(wgsl::voxelize::BOX.to_owned() + populate_source)),
            ),
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        settings: &Settings,
        line: &LineBuffer,
    ) {
        line.culling().clear(cmd);
        line.clear_offset(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Populate"),
            ..Default::default()
        });

        let n = line.occupancy().resolution().div_ceil(4);

        pass.set_pipeline(&self.scan);
        pass.set_bind_group(0, line.culling().pyramid().binding_write(), &[]);
        pass.set_bind_group(1, line.culling().binding_write(), &[]);
        pass.set_bind_group(2, line.occupancy().count().binding(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(match settings.voxelization {
            LineVoxelizationMode::Line => &self.populate_line,
            LineVoxelizationMode::Box => &self.populate_box,
            LineVoxelizationMode::Tube => &self.populate_tube,
        });

        pass.set_bind_group(0, line.culling().binding_write(), &[]);
        pass.set_bind_group(1, line.culling().pyramid().binding(), &[]);
        pass.set_bind_group(2, line.binding_render(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(settings.workgroups, 1, 1);
    }
}
