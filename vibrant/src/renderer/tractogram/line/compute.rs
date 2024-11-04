use crate::{
    asset::Tractogram,
    gpu::Gpu,
    renderer::{constants::Constants, environment::Environment, services::indirect::Indirect},
    surface::{Frame, Surface},
};
use std::any::type_name;
use wgpu::{
    CommandEncoder, ComputePassDescriptor, ComputePipeline, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp,
};

pub struct TractogramLineComputeRenderer {
    constants: Constants,
    clear: ComputePipeline,
    cull: ComputePipeline,
    set_dispatch_count: ComputePipeline,
    rasterize: ComputePipeline,
    shade: RenderPipeline,
    indirect: Indirect,
}

impl TractogramLineComputeRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let module = gpu.shader(
            &(Environment::wgsl() + include_str!("compute.wgsl")),
            Some(constants),
        );

        let layout_cull = gpu.pipeline_layout(&[
            &Tractogram::layout_full(gpu),
            &Environment::layout(gpu),
            &Surface::visibility(gpu),
            &Indirect::layout(gpu),
        ]);

        let layout = gpu.pipeline_layout(&[
            &Tractogram::layout_full(gpu),
            &Environment::layout(gpu),
            &Surface::visibility(gpu),
        ]);

        Self {
            constants: constants.clone(),
            clear: gpu.compute(&layout, &module, "clear"),
            cull: gpu.compute(&layout_cull, &module, "cull"),
            set_dispatch_count: gpu.compute(&layout_cull, &module, "set_dispatch_count"),
            rasterize: gpu.compute(&layout, &module, "rasterize"),
            shade: gpu.quad(
                &layout,
                &module,
                "shade",
                Surface::color_srgb_target(),
                None,
            ),
            indirect: Indirect::new(gpu, [0, 1, 1]),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
    ) {
        self.cull(cmd, environment, frame, tractogram);
        self.rasterize(cmd, environment, frame, tractogram);
        self.shade(cmd, environment, frame, tractogram);
    }

    fn cull(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
    ) {
        self.indirect.clear(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, tractogram.binding_full(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(2, frame.visibility(), &[]);
        pass.set_bind_group(3, self.indirect.binding(), &[]);

        let count = tractogram
            .vertex_count()
            .div_ceil(self.constants.workgroup_x);
        pass.set_pipeline(&self.cull);
        pass.dispatch_workgroups(count, 1, 1);

        pass.set_pipeline(&self.set_dispatch_count);
        pass.dispatch_workgroups(1, 1, 1);
    }

    fn rasterize(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, tractogram.binding_full(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(2, frame.visibility(), &[]);

        let (x, y) = self.constants.num_workgroups_surface();
        pass.set_pipeline(&self.clear);
        pass.dispatch_workgroups(x, y, 1);

        pass.set_pipeline(&self.rasterize);
        pass.dispatch_workgroups_indirect(&self.indirect.indirect(), 0);
    }

    fn shade(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame.color_srgb(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.shade);
        pass.set_bind_group(0, tractogram.binding_full(), &[]);
        pass.set_bind_group(1, environment.binding(), &[]);
        pass.set_bind_group(2, frame.visibility(), &[]);
        pass.draw(0..4, 0..1);
    }
}
