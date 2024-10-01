use std::sync::Arc;
use vibrant::controller::event::MouseButton;
use vibrant::Vec2;
use web_time::Instant;

use vibrant::controller::{event::Event, Controller};
use vibrant::renderer::Renderer;
use winit::event::MouseScrollDelta;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{self, WindowId},
};

struct App {
    window: Option<Arc<window::Window>>,
    egui: Option<egui_winit::State>,
    instant: Instant,
    renderer: Renderer,
    controller: Controller,
}

impl App {
    async fn new() -> Self {
        Self {
            window: None,
            egui: None,
            instant: Instant::now(),
            renderer: Renderer::new().await,
            controller: Controller::new(),
        }
    }

    fn event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let egui = self.egui.as_mut().expect("Egui");
        let window = self.window.as_ref().expect("Window");

        let consumed_by_egui = egui.on_window_event(window, &event).consumed;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(_) => self.request_redraw(),
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let instant = Instant::now();
                let duration = instant - self.instant;
                let dt = duration.as_secs_f32();
                self.instant = instant;

                let input = egui.take_egui_input(window);
                let output = egui
                    .egui_ctx()
                    .run(input, |ctx| self.controller.ui(ctx, dt));
                egui.handle_platform_output(&window, output.platform_output.clone());

                self.controller.update(dt);

                self.renderer.update(&self.controller);
                self.renderer.render(egui.egui_ctx(), output);

                self.request_redraw();
            }
            _ => (),
        }

        if let Some(vibrant_event) = vibrant_event(event) {
            if !consumed_by_egui {
                self.controller.event(vibrant_event);
            }
        }
    }

    fn window(&self) -> &Arc<window::Window> {
        self.window.as_ref().expect("Window was uninitialized")
    }

    fn request_redraw(&self) {
        self.window().request_redraw();
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

        let egui = egui_winit::State::new(
            egui::Context::default(),
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        self.renderer.create_surface(Arc::clone(&window));

        self.window = Some(window);
        self.egui = Some(egui);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.event(event_loop, event);
    }
}

fn vibrant_event(event: WindowEvent) -> Option<Event> {
    match event {
        WindowEvent::Resized(size) => Some(Event::Resized(Vec2::new(
            size.width as f32,
            size.height as f32,
        ))),
        WindowEvent::CursorMoved {
            device_id: _,
            position,
        } => Some(Event::MouseMoved(Vec2::new(
            position.x as f32,
            position.y as f32,
        ))),
        WindowEvent::MouseInput {
            device_id: _,
            state: winit::event::ElementState::Pressed,
            button: winit::event::MouseButton::Left,
        } => Some(Event::MousePressed(MouseButton::Left)),
        WindowEvent::MouseInput {
            device_id: _,
            state: winit::event::ElementState::Released,
            button: winit::event::MouseButton::Left,
        } => Some(Event::MouseReleased(MouseButton::Left)),
        WindowEvent::MouseInput {
            device_id: _,
            state: winit::event::ElementState::Pressed,
            button: winit::event::MouseButton::Right,
        } => Some(Event::MousePressed(MouseButton::Right)),
        WindowEvent::MouseInput {
            device_id: _,
            state: winit::event::ElementState::Released,
            button: winit::event::MouseButton::Right,
        } => Some(Event::MouseReleased(MouseButton::Right)),
        WindowEvent::MouseWheel {
            device_id: _,
            delta: MouseScrollDelta::PixelDelta(delta),
            phase: _,
        } => Some(Event::MouseWheel(Vec2::new(delta.x as f32, delta.y as f32))),
        _ => None,
    }
}

pub async fn run() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new().await;

    event_loop.run_app(&mut app).unwrap();
}
