/// Anti-aliasing pipeline module.
///
/// Sits between post-processing and display in the rendering pipeline:

use wgpu::{CommandEncoder, Extent3d, Origin3d, RenderPassDescriptor, RenderPipeline, TextureAspect};

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

    // TAA pipeline
    taa: RenderPipeline,
    taa_history_valid: bool,
    taa_last_width: u32,
    taa_last_height: u32,
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

        // TAA pipeline
        let taa = gpu.quad(
            "AA::TAA",
            &gpu.pipeline_layout(&[
                &Environment::layout(gpu),
                &ColorBuffer::layout(gpu), // current frame (frame.post)
                &ColorBuffer::layout(gpu), // history buffer (frame.taa_history)
            ]),
            ColorBuffer::target(),
            &gpu.shader(include_str!("taa.wgsl")),
        );

        Self {
            passthrough,
            smaa_edge,
            smaa_blend,
            smaa_neighborhood,
            taa,
            taa_history_valid: false,
            taa_last_width: 0,
            taa_last_height: 0,
        }
    }

    pub fn reset_taa_history(&mut self) {
        self.taa_history_valid = false;
    }

    pub fn dispatch(
        &mut self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
        settings: &Settings,
    ) {
        // When Adaptive is active, effective_aa_mode holds the resolved mode.
        let effective = settings.effective_aa_mode;
        match effective {
            AntiAliasingMode::Off | AntiAliasingMode::SSAA | AntiAliasingMode::Adaptive => {
                self.dispatch_passthrough(cmd, environment, frame)
            }
            AntiAliasingMode::SMAA => self.dispatch_smaa(cmd, environment, frame),
            AntiAliasingMode::TAA => self.dispatch_taa(cmd, environment, frame),
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

    fn dispatch_taa(
        &mut self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        frame: &Frame,
    ) {
        // auto-invalidate history if frame dimensions changed
        let w = frame.aa().width();
        let h = frame.aa().height();
        if w != self.taa_last_width || h != self.taa_last_height {
            self.taa_history_valid = false;
            self.taa_last_width = w;
            self.taa_last_height = h;
        }

        if !self.taa_history_valid {
            // no valid history yet
            self.dispatch_passthrough(cmd, environment, frame);
        } else {
            // blend current frame with history
            let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
                label: Some("AA::TAA"),
                color_attachments: &[Some(frame.aa().attachment_clear())],
                ..Default::default()
            });

            pass.set_pipeline(&self.taa);
            pass.set_bind_group(0, environment.binding(), &[]);
            pass.set_bind_group(1, frame.post().binding(), &[]);
            pass.set_bind_group(2, frame.taa_history().binding(), &[]);
            pass.draw(0..4, 0..1);
        }

        // copy frame.aa → frame.taa_history for next frame's reprojection
        cmd.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: frame.aa().texture(),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: frame.taa_history().texture(),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: frame.aa().width(),
                height: frame.aa().height(),
                depth_or_array_layers: 1,
            },
        );

        self.taa_history_valid = true;
    }
}
