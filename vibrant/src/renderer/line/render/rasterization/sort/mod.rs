use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::line::LineSet,
    controller::settings::Settings,
    gpu::Gpu,
    renderer::environment::Environment,
    sort::{KeyValuePair, SortPipeline, SortPipelineRadix},
};

pub struct LineRasterizationSortPipeline {
    compute: ComputePipeline,
    sort: SortPipeline,
}

impl LineRasterizationSortPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let compute = gpu.compute(
            "LineSort::Depth",
            &gpu.pipeline_layout(&[
                &LineSet::layout(gpu, true),
                &KeyValuePair::layout(gpu),
                &Environment::layout(gpu),
            ]),
            &gpu.shader(include_str!("depth.wgsl")),
        );

        let sort = SortPipeline::new(gpu, "u32");

        Self { compute, sort }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        settings: &Settings,
        line: &LineSet,
    ) {
        self.depth(cmd, environment, settings, line);

        self.sort.dispatch(
            cmd,
            settings.workgroups,
            line.sorted().ping().binding(),
            line.sorted().pong().binding(),
            line.sorted().ping().count(),
            SortPipelineRadix::R32,
        );
    }

    fn depth(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        settings: &Settings,
        line: &LineSet,
    ) {
        line.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("LineSort::Depth"),
            ..Default::default()
        });

        pass.set_pipeline(&self.compute);
        pass.set_bind_group(0, line.binding(true), &[]);
        pass.set_bind_group(1, line.sorted().ping().binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(settings.workgroups, 1, 1);
    }
}
