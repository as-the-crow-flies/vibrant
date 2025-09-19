use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::LineSet,
        texture::{MipTexture3D, R32Float},
    },
    controller::settings::LineVoxelizationMode,
    gpu::Gpu,
    renderer::{
        environment::Environment,
        wgsl::voxelize::{BOX, LINE, TUBE},
    },
    sort::{KeyValuePair, SortPipeline, SortPipelineRadix},
    surface::{vrc::VrcBuffer, Frame},
};

pub struct LineOccupancyAltPipeline {
    voxelize_tube: ComputePipeline,
    voxelize_line: ComputePipeline,
    voxelize_box: ComputePipeline,
    sort: SortPipeline,
    scan: ComputePipeline,
    clear: ComputePipeline,
    occupancy: ComputePipeline,
    mipmap: ComputePipeline,
}

impl LineOccupancyAltPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let voxelize_layout = &gpu.pipeline_layout(&[
            &LineSet::layout(gpu, true),
            &KeyValuePair::layout(gpu),
            &Environment::layout(gpu),
        ]);

        let voxelize_shader_source = include_str!("voxelize.wgsl");

        Self {
            clear: gpu.compute(
                "VrcVoxelizationPipeline::Clear",
                &gpu.pipeline_layout(&[
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &VrcBuffer::layout(gpu),
                ]),
                &gpu.shader(include_str!("clear.wgsl")),
            ),
            voxelize_tube: gpu.compute(
                "OccupancyAlt::Voxelize::Tube",
                voxelize_layout,
                &gpu.shader(&(TUBE.to_owned() + voxelize_shader_source)),
            ),
            voxelize_line: gpu.compute(
                "OccupancyAlt::Voxelize::Line",
                voxelize_layout,
                &gpu.shader(&(LINE.to_owned() + voxelize_shader_source)),
            ),
            voxelize_box: gpu.compute(
                "OccupancyAlt::Voxelize::Box",
                voxelize_layout,
                &gpu.shader(&(BOX.to_owned() + voxelize_shader_source)),
            ),
            sort: SortPipeline::new(gpu, "u32"),
            scan: gpu.compute(
                "OccupancyAlt::Scan",
                &gpu.pipeline_layout(&[&KeyValuePair::layout(gpu), &VrcBuffer::layout(gpu)]),
                &gpu.shader(include_str!("scan.wgsl")),
            ),

            occupancy: gpu.compute(
                "VrcVoxelizationPipeline::Occupancy",
                &gpu.pipeline_layout(&[
                    &LineSet::layout(gpu, true),
                    &KeyValuePair::layout(gpu),
                    &VrcBuffer::layout(gpu),
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("occupancy.wgsl")),
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
        setting: LineVoxelizationMode,
        line: &LineSet,
    ) {
        self.clear(cmd, frame);
        self.voxelize(cmd, environment, setting, line);
        self.sort.dispatch(
            cmd,
            line.vrc().ping().binding(),
            line.vrc().pong().binding(),
            line.vrc().ping().count(),
            SortPipelineRadix::R32,
        );
        self.scan(cmd, frame, line);
        self.occupancy(cmd, frame, environment, line);
    }

    fn voxelize(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        setting: LineVoxelizationMode,
        line: &LineSet,
    ) {
        line.clear_count(cmd);
        line.vrc().ping().clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        pass.set_bind_group(0, line.binding(true), &[]);
        pass.set_bind_group(1, line.vrc().ping().binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);

        pass.set_pipeline(match setting {
            LineVoxelizationMode::Line => &self.voxelize_line,
            LineVoxelizationMode::Box => &self.voxelize_box,
            LineVoxelizationMode::Tube => &self.voxelize_tube,
        });
        pass.dispatch_workgroups(18, 1, 1);
    }

    fn clear(&self, cmd: &mut CommandEncoder, frame: &Frame) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        let n = frame.occupancy().pyramid().resolution().div_ceil(8);

        pass.set_pipeline(&self.clear);
        pass.set_bind_group(0, frame.occupancy().pyramid().binding_write(), &[]);
        pass.set_bind_group(1, frame.vrc().binding(), &[]);
        pass.dispatch_workgroups(n, n, n);
    }

    fn scan(&self, cmd: &mut CommandEncoder, frame: &Frame, line: &LineSet) {
        line.vrc().ping().clear_offset(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        pass.set_pipeline(&self.scan);
        pass.set_bind_group(0, line.vrc().ping().binding(), &[]);
        pass.set_bind_group(1, frame.vrc().binding(), &[]);
        pass.dispatch_workgroups(18, 1, 1);
    }

    fn occupancy(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        line: &LineSet,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        let mut n = frame.occupancy().pyramid().resolution().div_ceil(8);

        pass.set_pipeline(&self.occupancy);
        pass.set_bind_group(0, line.binding(true), &[]);
        pass.set_bind_group(1, line.vrc().ping().binding(), &[]);
        pass.set_bind_group(2, frame.vrc().binding(), &[]);
        pass.set_bind_group(3, frame.occupancy().pyramid().binding_write(), &[]);
        pass.set_bind_group(4, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        for binding in frame.occupancy().pyramid().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(n, n, n);

            n = n.div_ceil(2);
        }
    }
}
