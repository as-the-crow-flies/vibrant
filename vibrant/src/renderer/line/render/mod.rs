use std::{any::type_name, cell::Cell};

use wgpu::{
    ColorTargetState, CommandEncoder, FragmentState, MultisampleState, PrimitiveState,
    PrimitiveTopology, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    asset::{
        environment::Environment, line::LineBuffer, radiance::GaussianRadianceBuffer,
        tractography::Tractography, Asset,
    },
    controller::{settings::RenderMode, Controller},
    gpu::Gpu,
    renderer::lighting::VMM_SIZE_OPTIONS,
    surface::{color::ColorBuffer, Frame},
};

/// Tractography ray-marcher. Shades line surfaces with the shared VMM Disney
/// BRDF (`trace.wgsl`) against the `radiance_lines` cascade. Two precompiled
/// pipeline sets, one variant each per VMM lobe count:
///
/// * **Combined** (`opaque.wgsl`): nearest hit only, alpha forced to 1, writes
///   HDR colour + the near→far hit fraction the volume tracer clamps to.
/// * **Overlay** (`transparent.wgsl`): every crossed line accumulated
///   (premultiplied), honoring `settings.alpha`; composited over the volume.
pub struct LineRenderPipeline {
    combined: [RenderPipeline; 3],
    overlay: [RenderPipeline; 3],
    active: Cell<usize>,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let trace = include_str!("trace.wgsl");

        let layout = gpu.pipeline_layout(&[
            &LineBuffer::layout_trace(gpu),
            &Environment::layout(gpu),
            &GaussianRadianceBuffer::layout(gpu),
            &Tractography::layout(gpu),
        ]);

        // Both variants are MRT (colour + R32Float depth) so the single
        // `fragment` entry point works for either; only target-0 blending and
        // the appended visit/result differ. Overlay ignores the depth output.
        let build = |appended: &str, blend: ColorTargetState| {
            VMM_SIZE_OPTIONS.map(|vmm_size| {
                let src =
                    (trace.to_string() + appended).replace("#VMM_SIZE", &vmm_size.to_string());
                let module = gpu.shader(&src);

                gpu.device()
                    .create_render_pipeline(&RenderPipelineDescriptor {
                        label: Some(type_name::<Self>()),
                        layout: Some(&layout),
                        vertex: VertexState {
                            module: &module,
                            entry_point: Some("vertex"),
                            buffers: &[],
                            compilation_options: Default::default(),
                        },
                        primitive: PrimitiveState {
                            topology: PrimitiveTopology::TriangleStrip,
                            ..Default::default()
                        },
                        fragment: Some(FragmentState {
                            module: &module,
                            entry_point: Some("fragment"),
                            targets: &[Some(blend.clone()), Some(ColorBuffer::depth_target())],
                            compilation_options: Default::default(),
                        }),
                        multisample: MultisampleState::default(),
                        depth_stencil: None,
                        multiview_mask: None,
                        cache: None,
                    })
            })
        };

        Self {
            combined: build(include_str!("opaque.wgsl"), ColorBuffer::target_blend()),
            overlay: build(
                include_str!("opaque.wgsl"),
                ColorBuffer::target_premultiplied(),
            ),
            active: Cell::new(
                VMM_SIZE_OPTIONS
                    .iter()
                    .position(|&size| size == 32)
                    .unwrap(),
            ),
        }
    }

    fn set_vmm_size(&self, vmm_size: u32) {
        if let Some(index) = VMM_SIZE_OPTIONS.iter().position(|&size| size == vmm_size) {
            self.active.set(index);
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        asset: &Asset,
        controller: &Controller,
        frame: &Frame,
    ) {
        // Overlay+both -> the line-only cascade; otherwise the primary one.
        let radiance = asset.radiance_lines.as_ref().or(asset.radiance.as_ref());
        let (Some(line), Some(radiance)) = (&asset.line, radiance) else {
            return;
        };

        self.set_vmm_size(controller.radiance().lobes());

        let variants = match controller.settings().render_mode {
            RenderMode::Combined => &self.combined,
            RenderMode::Overlay => &self.overlay,
        };

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some("LineTrace"),
            color_attachments: &[
                Some(frame.color().attachment()),
                Some(frame.line_depth().attachment()),
            ],
            ..Default::default()
        });

        let viewport = controller.viewport();
        pass.set_viewport(
            viewport.min.x,
            viewport.min.y,
            viewport.width(),
            viewport.height(),
            0.0,
            1.0,
        );

        pass.set_pipeline(&variants[self.active.get()]);
        pass.set_bind_group(0, line.binding_trace(), &[]);
        pass.set_bind_group(1, asset.environment.binding(), &[]);
        pass.set_bind_group(2, radiance.binding(), &[]);
        pass.set_bind_group(3, asset.tractography.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
