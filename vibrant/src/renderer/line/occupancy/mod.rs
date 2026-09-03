use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::{occupancy::OccupancyBuffer, LineBuffer},
        texture::{MipTexture3D, R32Float, R32Uint},
        tractography::Tractography,
        Asset,
    },
    controller::{settings::LineVoxelizationMode, Controller},
    gpu::Gpu,
    renderer::wgsl,
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
            &LineBuffer::layout_render(gpu),
            &Tractography::layout(gpu),
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
                    &Tractography::layout(gpu),
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
        asset: &Asset,
        controller: &Controller,
        line: &LineBuffer,
    ) {
        let settings = controller.settings();

        line.occupancy().clear(cmd);
        line.clear_offset(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occupancy"),
            ..Default::default()
        });

        let n = line.occupancy().pyramid().resolution().div_ceil(4);

        pass.set_bind_group(0, line.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, line.binding_render(), &[]);
        pass.set_bind_group(2, asset.tractography.binding(), &[]);

        pass.set_pipeline(match settings.voxelization {
            LineVoxelizationMode::Line => &self.voxelize_line,
            LineVoxelizationMode::Box => &self.voxelize_box,
            LineVoxelizationMode::Tube => &self.voxelize_tube,
        });
        pass.dispatch_workgroups(settings.workgroups, 1, 1);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, line.occupancy().binding_write(), &[]);
        pass.set_bind_group(1, line.occupancy().pyramid().binding_write(), &[]);
        pass.set_bind_group(2, line.occupancy().count().binding_write(), &[]);
        pass.set_bind_group(3, asset.tractography.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = line.occupancy().pyramid().resolution().div_ceil(4);

        for binding in line.occupancy().pyramid().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
