use std::sync::Arc;
use vibrant::controller::event::{Key, MouseButton};
use vibrant::gpu::Gpu;
use vibrant::Vec2;
use web_time::Instant;

use vibrant::controller::{event::Event, Controller};
use vibrant::renderer::Renderer;
use winit::event::{ElementState, KeyEvent, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{self, WindowId},
};

struct App {
    gpu: Gpu,
    window: Option<Arc<window::Window>>,
    egui: Option<egui_winit::State>,
    renderer: Option<Renderer>,
    controller: Controller,
    focused: bool,
    fps: Fps<8>,
}

impl App {
    fn new(gpu: Gpu) -> Self {
        Self {
            gpu,
            window: None,
            egui: None,
            renderer: None,
            controller: Controller::new(),
            focused: true,
            fps: Fps::new(),
        }
    }

    fn event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let egui = self.egui.as_mut().expect("Egui");
        let window = self.window.as_ref().expect("Window");
        let renderer = self.renderer.as_mut().expect("Renderer");

        let consumed_by_egui = egui.on_window_event(window, &event).consumed;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.focused = focused;

                if focused {
                    self.request_redraw()
                }
            }
            WindowEvent::Resized(size) => self.controller.resize(size),
            WindowEvent::RedrawRequested => {
                self.fps.tick();

                let input = egui.take_egui_input(window);
                let output = egui
                    .egui_ctx()
                    .run(input, |ctx| self.controller.ui(ctx, self.fps.seconds()));
                egui.handle_platform_output(&window, output.platform_output.clone());

                renderer.render(&self.gpu, &self.controller, egui.egui_ctx(), output);

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
        attributes = attributes.with_title("VIBRANT").with_maximized(true);

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

        let renderer = Renderer::new(&self.gpu, Arc::clone(&window));

        let egui = egui_winit::State::new(
            egui::Context::default(),
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.egui = Some(egui);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.event(event_loop, event);
    }
}

fn vibrant_event(event: WindowEvent) -> Option<Event> {
    match event {
        WindowEvent::Resized(size) => Some(Event::Resized(size.width, size.height)),
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
        WindowEvent::MouseInput {
            device_id: _,
            state: winit::event::ElementState::Pressed,
            button: winit::event::MouseButton::Middle,
        } => Some(Event::MousePressed(MouseButton::Middle)),
        WindowEvent::MouseInput {
            device_id: _,
            state: winit::event::ElementState::Released,
            button: winit::event::MouseButton::Middle,
        } => Some(Event::MouseReleased(MouseButton::Middle)),
        WindowEvent::MouseWheel {
            device_id: _,
            delta: MouseScrollDelta::PixelDelta(delta),
            phase: _,
        } => Some(Event::MouseWheel(Vec2::new(
            0.01 * delta.x as f32,
            0.01 * delta.y as f32,
        ))),
        WindowEvent::MouseWheel {
            device_id: _,
            delta: MouseScrollDelta::LineDelta(x, y),
            phase: _,
        } => Some(Event::MouseWheel(Vec2::new(x, y))),
        WindowEvent::KeyboardInput {
            device_id: _,
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    logical_key: _,
                    text: _,
                    location: _,
                    state: ElementState::Pressed,
                    repeat: _,
                    ..
                },
            is_synthetic: _,
        } => keycode(code).map(|key| Event::KeyPressed(key)),
        WindowEvent::KeyboardInput {
            device_id: _,
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    logical_key: _,
                    text: _,
                    location: _,
                    state: ElementState::Released,
                    repeat: _,
                    ..
                },
            is_synthetic: _,
        } => keycode(code).map(|key| Event::KeyReleased(key)),
        _ => None,
    }
}

fn keycode(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::ShiftLeft => Some(Key::Shift),
        KeyCode::ShiftRight => Some(Key::Shift),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::NumpadBackspace => Some(Key::Backspace),
        _ => None,
    }
}

pub async fn run() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(Gpu::new().await);

    #[cfg(not(target_arch = "wasm32"))]
    {
        event_loop.run_app(&mut app).unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
}

pub struct Fps<const N: usize> {
    buffer: [Instant; N],
    index: usize,
}

impl<const N: usize> Fps<N> {
    pub fn new() -> Self {
        Self {
            buffer: [Instant::now(); N],
            index: 0,
        }
    }

    pub fn tick(&mut self) {
        self.buffer[self.index] = Instant::now();
        self.index = (self.index + 1) % N;
    }

    pub fn seconds(&self) -> f32 {
        let instants: Vec<Instant> = (1..=N).map(|i| self.buffer[(self.index + i) % N]).collect();

        let total: f32 = instants
            .windows(2)
            .map(|window| (window[1] - window[0]).as_secs_f32())
            .sum();

        total / N as f32
    }
}
