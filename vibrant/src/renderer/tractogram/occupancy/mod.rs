use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::scalar::{R16Uint, R8Unorm, ScalarTexture3D},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{occupancy::Occupancy, Frame},
};

pub struct OccupancyPipeline {
    bin: ComputePipeline,
    threshold: ComputePipeline,
    occupancy: ComputePipeline,
    mipmap: ComputePipeline,
}

impl OccupancyPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            bin: gpu.compute(
                "Occupancy::Bin",
                &gpu.pipeline_layout(&[
                    &Occupancy::layout(gpu, false),
                    &ScalarTexture3D::<R16Uint>::layout(gpu),
                    &ScalarTexture3D::<R8Unorm>::layout(gpu),
                ]),
                &gpu.shader(include_str!("bin.wgsl")),
            ),
            threshold: gpu.compute(
                "Occupancy::Threshold",
                &gpu.pipeline_layout(&[&Occupancy::layout(gpu, false), &Environment::layout(gpu)]),
                &gpu.shader(include_str!("threshold.wgsl")),
            ),
            occupancy: gpu.compute(
                "Occupancy::Occupancy",
                &gpu.pipeline_layout(&[
                    &Occupancy::layout(gpu, false),
                    &ScalarTexture3D::<R16Uint>::layout(gpu),
                    &ScalarTexture3D::<R8Unorm>::layout(gpu),
                    &ScalarTexture3D::<R8Unorm>::layout_write(gpu),
                ]),
                &gpu.shader(include_str!("occupancy.wgsl")),
            ),
            mipmap: gpu.compute(
                "Occupancy::Mipmap",
                &gpu.pipeline_layout(&[&ScalarTexture3D::<R8Unorm>::layout_mipmap(gpu)]),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    pub fn render(&self, cmd: &mut CommandEncoder, frame: &Frame, environment: &Environment) {
        frame.occupancy().clear(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Occupancy"),
            ..Default::default()
        });

        let n = frame.density().resolution().div_ceil(8);

        pass.set_pipeline(&self.bin);
        pass.set_bind_group(0, frame.occupancy().binding(false), &[]);
        pass.set_bind_group(1, frame.density().count().binding(), &[]);
        pass.set_bind_group(2, frame.occlusion().texture().binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.threshold);
        pass.set_bind_group(0, frame.occupancy().binding(false), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.dispatch_workgroups(1, 1, 1);

        pass.set_pipeline(&self.occupancy);
        pass.set_bind_group(0, frame.occupancy().binding(false), &[]);
        pass.set_bind_group(1, frame.density().count().binding(), &[]);
        pass.set_bind_group(2, frame.occlusion().texture().binding(), &[]);
        pass.set_bind_group(3, frame.occupancy().texture().binding_write(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = frame.occupancy().texture().size().div_ceil(8);
        for binding in frame.occupancy().texture().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
