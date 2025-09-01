use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        line::LineSet,
        texture::{MipTexture3D, R32Float},
    },
    gpu::Gpu,
    renderer::environment::Environment,
    sort::{KeyValuePair, SortPipeline, SortPipelineRadix},
    surface::{vrc::VrcBuffer, Frame},
};

pub struct VrcLineVoxelizationPipeline {
    quantize: ComputePipeline,
    sort: SortPipeline,
    scan: ComputePipeline,
    clear: ComputePipeline,
    occupancy: ComputePipeline,
    mipmap: ComputePipeline,
}

impl VrcLineVoxelizationPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            quantize: gpu.compute(
                "VrcVoxelizationPipeline::Quantize",
                &gpu.pipeline_layout(&[
                    &LineSet::layout(gpu, false),
                    &KeyValuePair::layout(gpu),
                    &Environment::layout(gpu),
                ]),
                &gpu.shader(include_str!("quantize.wgsl")),
            ),
            sort: SortPipeline::new(gpu),
            scan: gpu.compute(
                "VrcVoxelizationPipeline::Scan",
                &gpu.pipeline_layout(&[&KeyValuePair::layout(gpu), &VrcBuffer::layout(gpu)]),
                &gpu.shader(include_str!("scan.wgsl")),
            ),
            clear: gpu.compute(
                "VrcVoxelizationPipeline::Clear",
                &gpu.pipeline_layout(&[
                    &MipTexture3D::<R32Float>::layout_write(gpu),
                    &VrcBuffer::layout(gpu),
                ]),
                &gpu.shader(include_str!("clear.wgsl")),
            ),
            occupancy: gpu.compute(
                "VrcVoxelizationPipeline::Occupancy",
                &gpu.pipeline_layout(&[
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
        line: &LineSet,
    ) {
        self.clear(cmd, frame);
        self.quantize(cmd, environment, line);
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

    fn clear(&self, cmd: &mut CommandEncoder, frame: &Frame) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        let n = frame.occupancy().density().resolution().div_ceil(8);

        pass.set_pipeline(&self.clear);
        pass.set_bind_group(0, frame.occupancy().density().binding_write(), &[]);
        pass.set_bind_group(1, frame.vrc().binding(), &[]);
        pass.dispatch_workgroups(n, n, n);
    }

    fn quantize(&self, cmd: &mut CommandEncoder, environment: &Environment, line: &LineSet) {
        line.vrc().ping().clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            ..Default::default()
        });

        pass.set_pipeline(&self.quantize);
        pass.set_bind_group(0, line.binding(false), &[]);
        pass.set_bind_group(1, line.vrc().ping().binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(line.len().div_ceil(1024), 1, 1);
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

        let mut n = frame.occupancy().density().resolution().div_ceil(8);

        pass.set_pipeline(&self.occupancy);
        pass.set_bind_group(0, line.vrc().ping().binding(), &[]);
        pass.set_bind_group(1, frame.vrc().binding(), &[]);
        pass.set_bind_group(2, frame.occupancy().density().binding_write(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        for binding in frame.occupancy().density().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(n, n, n);

            n = n.div_ceil(2);
        }
    }
}
