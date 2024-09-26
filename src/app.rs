pub mod gpu;
pub mod loader;
pub mod renderer;
pub mod surface;

use std::sync::Arc;

use gpu::Gpu;
use log::info;
use renderer::Renderer;
use surface::Surface;
use wgpu::TextureViewDescriptor;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct App {
    gpu: Gpu,
    renderer: Renderer,
    window: Option<Arc<Window>>,
    surface: Option<Surface>,
}

impl App {
    async fn new() -> App {
        let gpu = Gpu::new().await;

        Self {
            renderer: Renderer::new(&gpu),
            gpu,
            window: None,
            surface: None,
        }
    }

    fn redraw(&self) {
        self.window.as_ref().unwrap().request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut attributes = Window::default_attributes();

        let size = LogicalSize::new(800, 600);

        attributes = attributes.with_title("VIBRANT").with_inner_size(size);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use web_sys::{window, HtmlCanvasElement};
            use winit::platform::web::WindowAttributesExtWebSys;

            let mut canvas = window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("canvas"))
                .expect("No Element with id 'canvas'")
                .dyn_into::<HtmlCanvasElement>()
                .expect("No Element of type canvas");

            canvas.set_width(size.width);
            canvas.set_height(size.height);

            attributes = attributes.with_canvas(Some(canvas));
        }

        let window = Arc::new(event_loop.create_window(attributes).unwrap());

        let surface = Surface::new(&self.gpu, Arc::clone(&window), size.width, size.height);

        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Focused(focused) => {
                if focused {
                    self.redraw();
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.surface
                    .as_ref()
                    .unwrap()
                    .configure(&self.gpu, size.width, size.height);

                info!("{:?}", size);

                self.redraw()
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => self.redraw(),
            WindowEvent::RedrawRequested => {
                let texture = self.surface.as_ref().unwrap().get_current_texture();

                let view = texture.texture.create_view(&TextureViewDescriptor {
                    label: Some("surface view"),
                    format: Some(Surface::VIEW_FORMAT),
                    ..Default::default()
                });

                self.renderer.render(&self.gpu, &view);

                texture.present();
            }
            _ => (),
        }
    }
}

pub async fn run() {
    let mut app = App::new().await;

    EventLoop::new().unwrap().run_app(&mut app).unwrap();
}
