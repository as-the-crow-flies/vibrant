use std::any::type_name;

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
    renderer::lighting::VmmVariants,
    surface::{color::ColorBuffer, Frame},
};

/// Tractography ray-marcher. Shades line surfaces with the shared VMM Disney
/// BRDF (`trace.wgsl`) against the `radiance_lines` cascade. Two pipeline
/// sets, one lazily compiled variant each per VMM lobe count:
///
/// * **Combined** (`opaque.wgsl`): nearest hit only, alpha forced to 1, writes
///   HDR colour + the near→far hit fraction the volume tracer clamps to.
/// * **Overlay** (`transparent.wgsl`): every crossed line accumulated
///   (premultiplied), honoring `settings.alpha`; composited over the volume.
pub struct LineRenderPipeline {
    combined: VmmVariants<RenderPipeline>,
    overlay: VmmVariants<RenderPipeline>,
}

impl LineRenderPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = gpu.pipeline_layout(&[
            &LineBuffer::layout_trace(gpu),
            &Environment::layout(gpu),
            &GaussianRadianceBuffer::layout(gpu),
            &Tractography::layout(gpu),
        ]);

        // Both variants are MRT (colour + R32Float depth) so the single
        // `fragment` entry point works for either; only target-0 blending and
        // the visit/result in `shader` differ. Overlay ignores the depth output.
        let build = |label: &'static str, shader: &'static str, blend: ColorTargetState| {
            let layout = layout.clone();
            VmmVariants::new(gpu, label, move |gpu, vmm_size| {
                let module = gpu.shader_with(shader, &[("VMM_SIZE", &vmm_size)]);

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
            combined: build(
                "LineTrace::Combined",
                "renderer/line/render/trace_opaque.wgsl",
                ColorBuffer::target_blend(),
            ),
            overlay: build(
                "LineTrace::Overlay",
                "renderer/line/render/trace_opaque.wgsl",
                ColorBuffer::target_premultiplied(),
            ),
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

        let variants = match controller.settings().render_mode {
            RenderMode::Combined => &self.combined,
            RenderMode::Overlay => &self.overlay,
        };
        variants.set_vmm_size(controller.radiance().lobes());

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

        pass.set_pipeline(variants.active());
        pass.set_bind_group(0, line.binding_trace(), &[]);
        pass.set_bind_group(1, asset.environment.binding(), &[]);
        pass.set_bind_group(2, radiance.binding(), &[]);
        pass.set_bind_group(3, asset.tractography.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}
