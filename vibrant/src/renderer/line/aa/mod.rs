/// Anti-aliasing pipeline module.
///
/// Sits between post-processing and display in the rendering pipeline:

use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    controller::settings::{AntiAliasingMode, Settings},
    gpu::Gpu,
    renderer::environment::Environment,
    surface::{color::ColorBuffer, Frame},
};

/// AA render pipelines
pub struct AntiAliasingPipeline {
    passthrough: RenderPipeline,

    // SMAA pipelines
    // Pass 1: Luma edge detection
    smaa_edge: RenderPipeline,
    // Pass 2: Blend weight calculation
    smaa_blend: RenderPipeline,
    // Pass 3: Neighborhood blending
    smaa_neighborhood: RenderPipeline,
}

impl AntiAliasingPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let passthrough = gpu.quad(
            "AA::Passthrough",
            &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
            ColorBuffer::target(),
            &gpu.shader(include_str!("passthrough.wgsl")),
        );

        // SMAA Pass 1
        let smaa_edge = gpu.quad(
            "AA::SMAA::EdgeDetection",
            &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
            ColorBuffer::target(),
            &gpu.shader(include_str!("smaa_edge.wgsl")),
        );

        // SMAA Pass 2
        let smaa_blend = gpu.quad(
            "AA::SMAA::BlendWeight",
            &gpu.pipeline_layout(&[&Environment::layout(gpu), &ColorBuffer::layout(gpu)]),
            ColorBuffer::target(),
            &gpu.shader(include_str!("smaa_blend.wgsl")),
        );

        // SMAA Pass 3
        let smaa_neighborhood = gpu.quad(
            "AA::SMAA::NeighborhoodBlend",
            &gpu.pipeline_layout(&[
                &Environment::layout(gpu),
                &ColorBuffer::layout(gpu), 
                &ColorBuffer::layout(gpu), 
            ]),
            ColorBuffer::target(),
            &gpu.shader(include_str!("smaa_neighborhood.wgsl")),
        );

        Self {
            passthrough,
            smaa_edge,
            smaa_blend,
            smaa_neighborhood,
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        settings: &Settings,
    ) {
        match settings.aa_mode {
            AntiAliasingMode::Off => self.dispatch_passthrough(cmd, environment, frame),
            AntiAliasingMode::SMAA => self.dispatch_smaa(cmd, environment, frame),
        }
    }

    fn dispatch_passthrough(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some("AA::Passthrough"),
            color_attachments: &[Some(frame.aa().attachment_clear())],
            ..Default::default()
        });

        pass.set_pipeline(&self.passthrough);
        pass.set_bind_group(0, environment.binding(), &[]);
        pass.set_bind_group(1, frame.post().binding(), &[]);
        pass.draw(0..4, 0..1);
    }

    fn dispatch_smaa(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
    ) {
        // Pass 1
        {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("AA::SMAA::EdgeDetection"),
                color_attachments: &[Some(frame.smaa_edges().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.smaa_edge);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.post().binding(), &[]);
            pass.draw(0..4, 0..1);
        }

        // Pass 2
        {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("AA::SMAA::BlendWeight"),
                color_attachments: &[Some(frame.smaa_blend().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.smaa_blend);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.smaa_edges().binding(), &[]);
            pass.draw(0..4, 0..1);
        }

        // Pass 3
        {
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("AA::SMAA::NeighborhoodBlend"),
                color_attachments: &[Some(frame.aa().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.smaa_neighborhood);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.post().binding(), &[]);
            pass.set_bind_group(2, frame.smaa_blend().binding(), &[]);
            pass.draw(0..4, 0..1);
        }
    }
}
