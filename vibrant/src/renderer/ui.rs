use std::any::type_name;

use egui_wgpu::RendererOptions;
use wgpu::{CommandEncoder, RenderPassDescriptor};

use crate::{
    gpu::Gpu,
    surface::{color::ColorBuffer, Frame},
};

pub struct UiRenderer {
    egui: egui_wgpu::Renderer,
}

impl UiRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            egui: egui_wgpu::Renderer::new(
                gpu.device(),
                // Own texture, decoupled from the swapchain: egui paints
                // sRGB-encoded premultiplied color that the present pass
                // composites after tone mapping.
                ColorBuffer::LDR_FORMAT,
                RendererOptions {
                    msaa_samples: 1,
                    depth_stencil_format: None,
                    dithering: false,
                    predictable_texture_filtering: false,
                },
            ),
        }
    }

    /// Applies pending texture updates/frees. Must be called for every [`egui::FullOutput`],
    /// even if [`Self::paint`] ends up being skipped for that frame (e.g. no surface texture
    /// available, or the frame is not meant to be presented), since `egui` expects every
    /// `TexturesDelta` to be fully handled before it's dropped.
    pub fn update_textures(&mut self, gpu: &Gpu, textures_delta: &egui::TexturesDelta) {
        let (device, queue) = (gpu.device(), gpu.queue());

        for (id, deltas) in &textures_delta.set {
            for image_delta in deltas {
                self.egui.update_texture(device, queue, *id, image_delta);
            }
        }

        for texture in &textures_delta.free {
            self.egui.free_texture(texture)
        }
    }

    pub fn paint(
        &mut self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        ctx: &egui::Context,
        shapes: Vec<egui::epaint::ClippedShape>,
        pixels_per_point: f32,
    ) {
        let (device, queue) = (gpu.device(), gpu.queue());

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [frame.overlay().width(), frame.overlay().height()],
            pixels_per_point,
        };

        let tris = ctx.tessellate(shapes, pixels_per_point);

        self.egui.update_buffers(device, queue, cmd, &tris, &screen);

        let mut pass = cmd
            .begin_render_pass(&RenderPassDescriptor {
                label: Some(type_name::<Self>()),
                color_attachments: &[Some(frame.overlay().attachment_clear())],
                ..Default::default()
            })
            .forget_lifetime();

        self.egui.render(&mut pass, &tris, &screen);
    }
}
