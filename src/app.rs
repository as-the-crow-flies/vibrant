pub mod controller;
pub mod gpu;
pub mod loader;
pub mod renderer;
pub mod surface;

use log::warn;
use std::sync::Arc;
use web_time::Instant;

use controller::Controller;
use renderer::Renderer;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{self, WindowId},
};

struct App {
    window: Option<Arc<window::Window>>,
    instant: Instant,
    renderer: Renderer,
    controller: Controller,
}

impl App {
    async fn new() -> Self {
        Self {
            window: None,
            instant: Instant::now(),
            renderer: Renderer::new().await,
            controller: Controller::new(),
        }
    }

    fn window(&self) -> &Arc<window::Window> {
        self.window.as_ref().expect("Window was uninitialized")
    }

    fn request_redraw(&self) {
        self.window().request_redraw();
    }

    fn run(&mut self) {
        EventLoop::new().unwrap().run_app(self).unwrap();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut attributes = window::Window::default_attributes();

        attributes = attributes.with_title("VIBRANT");

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

            canvas.set_width(1);
            canvas.set_height(1);

            attributes = attributes.with_canvas(Some(canvas));
        }

        let window = Arc::new(event_loop.create_window(attributes).unwrap());

        self.renderer.create_surface(Arc::clone(&window));
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(_) => self.request_redraw(),
            WindowEvent::Resized(size) => self.renderer.resize(size),
            WindowEvent::RedrawRequested => {
                let instant = Instant::now();
                let duration = instant - self.instant;

                self.controller.update(duration.as_secs_f32());
                self.instant = instant;

                self.renderer.render(&self.controller);

                self.request_redraw();
            }
            _ => (),
        }

        self.controller.event(event);
    }
}

pub async fn run() {
    App::new().await.run();
}
