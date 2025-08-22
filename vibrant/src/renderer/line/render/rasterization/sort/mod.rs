use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::line::LineSet,
    gpu::Gpu,
    renderer::environment::Environment,
    sort::{KeyValuePair, SortPipeline},
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

        let sort = SortPipeline::new(gpu);

        Self { compute, sort }
    }

    pub fn dispatch(&self, cmd: &mut CommandEncoder, environment: &Environment, line: &LineSet) {
        self.depth(cmd, environment, line);

        self.sort.dispatch(
            cmd,
            line.sorted().ping().binding(),
            line.sorted().pong().binding(),
            line.sorted().ping().count(),
        );
    }

    fn depth(&self, cmd: &mut CommandEncoder, environment: &Environment, line: &LineSet) {
        line.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("LineSort::Depth"),
            ..Default::default()
        });

        pass.set_pipeline(&self.compute);
        pass.set_bind_group(0, line.binding(true), &[]);
        pass.set_bind_group(1, line.sorted().ping().binding(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(18, 1, 1);
    }
}

#[cfg(test)]
mod test {
    use pollster::FutureExt;
    use wgpu::wgt::CommandEncoderDescriptor;

    use crate::{
        asset::line::LineSet,
        controller::Controller,
        file::LineFile,
        gpu::Gpu,
        renderer::{
            environment::Environment,
            line::{
                render::rasterization::sort::LineRasterizationSortPipeline,
                transform::LineTransformPipeline,
            },
        },
    };

    #[test]
    fn test() {
        let gpu = &Gpu::new().block_on();

        let line = LineSet::new(
            gpu,
            &LineFile::from_file("../assets/HCP-100307/TOM_trackings/AF_left.tck"),
        );

        let environment = Environment::new(&gpu);
        environment.update(&gpu, &Controller::new());

        let transform = LineTransformPipeline::new(gpu);
        let sort = LineRasterizationSortPipeline::new(gpu);

        let mut cmd = gpu
            .device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        transform.render(&mut cmd, &environment, &line);
        sort.dispatch(&mut cmd, &environment, &line);

        gpu.submit(cmd);
        gpu.wait();

        let indices: Vec<u32> = gpu.read_buffer(line.indices()).block_on();
        let depths_sorted: Vec<u32> = gpu.read_buffer(line.sorted().ping().value()).block_on();
        let indices_sorted: Vec<u32> = gpu.read_buffer(line.sorted().ping().key()).block_on();

        let count: Vec<u32> = gpu.read_buffer(line.count()).block_on();

        dbg!(&indices[0..64]);
        dbg!(&indices_sorted[0..64]);
        dbg!(&depths_sorted[0..64]);
        dbg!(&count);
    }
}
