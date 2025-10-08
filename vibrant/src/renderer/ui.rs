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
                ColorBuffer::FORMAT,
                RendererOptions {
                    msaa_samples: 1,
                    depth_stencil_format: None,
                    dithering: false,
                },
            ),
        }
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        ctx: &egui::Context,
        output: egui::FullOutput,
    ) {
        let (device, queue) = (gpu.device(), gpu.queue());

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [frame.color().width(), frame.color().height()],
            pixels_per_point: output.pixels_per_point,
        };

        let tris = ctx.tessellate(output.shapes, output.pixels_per_point);

        for (id, image_delta) in &output.textures_delta.set {
            self.egui.update_texture(device, queue, *id, image_delta);
        }

        self.egui.update_buffers(device, queue, cmd, &tris, &screen);

        let mut pass = cmd
            .begin_render_pass(&RenderPassDescriptor {
                label: Some(type_name::<Self>()),
                color_attachments: &[Some(frame.color().attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();

        self.egui.render(&mut pass, &tris, &screen);

        drop(pass);

        for texture in &output.textures_delta.free {
            self.egui.free_texture(texture)
        }
    }
}
