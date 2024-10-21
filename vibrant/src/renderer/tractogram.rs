use crate::{
    asset::{Density, Tractogram},
    surface::{Frame, Surface},
};
use std::any::type_name;

use wgpu::{
    CommandEncoder, ComputePassDescriptor, ComputePipeline, LoadOp, Operations,
    PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    StoreOp,
};

use crate::gpu::Gpu;

use super::{constants::Constants, environment::Environment};

pub struct TractogramComputeRenderer {
    constants: Constants,
    clear: ComputePipeline,
    rasterize: ComputePipeline,
    shade: RenderPipeline,
}

impl TractogramComputeRenderer {
    pub fn new(gpu: &Gpu, constants: &Constants) -> Self {
        let label = Some(type_name::<Self>());

        let module = gpu.shader(
            &(Environment::wgsl() + include_str!("wgsl/tractogram.wgsl")),
            Some(constants),
        );

        let layout = gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[
                    &Tractogram::layout_full(gpu),
                    &Density::layout_render(gpu),
                    &Environment::layout(gpu),
                    &Surface::visibility(gpu),
                ],
                push_constant_ranges: &[],
            });

        Self {
            constants: constants.clone(),
            clear: gpu.compute(&layout, &module, "clear"),
            rasterize: gpu.compute(&layout, &module, "rasterize"),
            shade: gpu.quad(
                &layout,
                &module,
                "shade",
                Surface::color_srgb_target(),
                None,
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
    ) {
        self.rasterize(cmd, environment, frame, tractogram, density);
        self.shade(cmd, environment, frame, tractogram, density);
    }

    fn rasterize(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(type_name::<Self>()),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, tractogram.binding_full(), &[]);
        pass.set_bind_group(1, density.binding_render(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, frame.visibility(), &[]);

        let (x, y) = self.constants.num_workgroups_surface();
        pass.set_pipeline(&self.clear);
        pass.dispatch_workgroups(x, y, 1);

        let count = tractogram.count().div_ceil(self.constants.workgroup_x);
        pass.set_pipeline(&self.rasterize);
        pass.dispatch_workgroups(count, 1, 1);
    }

    fn shade(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        tractogram: &Tractogram,
        density: &Density,
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
        pass.set_bind_group(1, density.binding_render(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, frame.visibility(), &[]);
        pass.draw(0..4, 0..1);
    }
}
