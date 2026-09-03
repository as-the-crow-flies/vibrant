use std::any::type_name;
use std::time::Duration;

use egui_wgpu::RendererOptions;
use wgpu::{CommandEncoder, RenderPassDescriptor};
use winit::{event::WindowEvent, window::Window};

use crate::{
    gpu::Gpu,
    surface::{color::ColorBuffer, Frame},
};

/// Draw lists from the last [`UiRenderer::run`], consumed by the next
/// [`UiRenderer::paint`].
struct Pending {
    shapes: Vec<egui::epaint::ClippedShape>,
    pixels_per_point: f32,
}

pub struct UiRenderer {
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    pending: Option<Pending>,
}

impl UiRenderer {
    pub fn new(gpu: &Gpu, window: &Window) -> Self {
        let ctx = egui::Context::default();

        let mut fonts = egui::FontDefinitions::default();
        crate::controller::icons::install(&mut fonts);
        ctx.set_fonts(fonts);

        Self {
            state: egui_winit::State::new(
                ctx,
                egui::viewport::ViewportId::ROOT,
                window,
                Some(window.scale_factor() as f32),
                None,
                None,
            ),
            renderer: egui_wgpu::Renderer::new(
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
            pending: None,
        }
    }

    pub fn on_window_event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
    ) -> egui_winit::EventResponse {
        self.state.on_window_event(window, event)
    }

    /// Runs the egui pass for this frame: feeds input, builds the UI via
    /// `build`, applies texture deltas, and stashes the draw lists for the next
    /// [`Self::paint`]. Returns egui's requested delay until the next repaint
    /// (`Duration::MAX` = none).
    pub fn run(
        &mut self,
        gpu: &Gpu,
        window: &Window,
        build: impl FnMut(&mut egui::Ui),
    ) -> Duration {
        let input = self.state.take_egui_input(window);
        let mut output = self.state.egui_ctx().run_ui(input, build);

        self.state
            .handle_platform_output(window, output.platform_output.clone());

        // Must happen even on frames we don't paint: `TexturesDelta` panics on
        // drop with deltas still unapplied.
        self.apply_textures(gpu, &output.textures_delta);
        output.textures_delta.clear();

        self.pending = Some(Pending {
            shapes: output.shapes,
            pixels_per_point: output.pixels_per_point,
        });

        output
            .viewport_output
            .values()
            .map(|viewport| viewport.repaint_delay)
            .min()
            .unwrap_or(Duration::MAX)
    }

    /// Paints the UI from the last [`Self::run`] into `frame`'s overlay. No-op
    /// if nothing is pending.
    pub fn paint(&mut self, gpu: &Gpu, cmd: &mut CommandEncoder, frame: &Frame) {
        let Some(Pending {
            shapes,
            pixels_per_point,
        }) = self.pending.take()
        else {
            return;
        };

        let (device, queue) = (gpu.device(), gpu.queue());

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [frame.overlay().width(), frame.overlay().height()],
            pixels_per_point,
        };

        let tris = self.state.egui_ctx().tessellate(shapes, pixels_per_point);

        self.renderer.update_buffers(device, queue, cmd, &tris, &screen);

        let mut pass = cmd
            .begin_render_pass(&RenderPassDescriptor {
                label: Some(type_name::<Self>()),
                color_attachments: &[Some(frame.overlay().attachment_clear())],
                ..Default::default()
            })
            .forget_lifetime();

        self.renderer.render(&mut pass, &tris, &screen);
    }

    fn apply_textures(&mut self, gpu: &Gpu, textures_delta: &egui::TexturesDelta) {
        let (device, queue) = (gpu.device(), gpu.queue());

        for (id, deltas) in &textures_delta.set {
            for image_delta in deltas {
                self.renderer.update_texture(device, queue, *id, image_delta);
            }
        }

        for texture in &textures_delta.free {
            self.renderer.free_texture(texture)
        }
    }
}
