use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use vibrant::controller::event::{Key, MouseButton};
use vibrant::file::FileStage;
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
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{self, WindowId},
};

struct App {
    gpu: Gpu,
    window: Option<Arc<window::Window>>,
    renderer: Option<Renderer>,
    controller: Controller,
    focused: bool,
    fps: Fps<8>,
    /// When set, `about_to_wait` schedules a redraw at this instant - used for
    /// egui's timed repaints (tooltips, animations) once the render loop has
    /// otherwise gone idle on a converged frame.
    next_repaint: Option<Instant>,
    /// Set while a frame's GPU work is in flight, cleared by
    /// `on_submitted_work_done`. `RedrawRequested` bails out while it's set so at
    /// most one frame is ever queued: on web `Gpu::wait` can't block, so without
    /// this an input burst at low fps stacks a render per animation frame and
    /// the view lags several frames behind.
    in_flight: Arc<AtomicBool>,
    /// A redraw is wanted (input, egui, resize, ...). Set from `event`, consumed
    /// by the next render.
    dirty: bool,
}

impl App {
    fn new(gpu: Gpu) -> Self {
        Self {
            gpu,
            window: None,
            renderer: None,
            controller: Controller::new(),
            focused: true,
            fps: Fps::new(),
            next_repaint: None,
            in_flight: Arc::new(AtomicBool::new(false)),
            dirty: false,
        }
    }

    fn event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let window = Arc::clone(self.window.as_ref().expect("Window"));
        let renderer = self.renderer.as_mut().expect("Renderer");

        let response = renderer.ui().on_window_event(&window, &event);
        let consumed = response.consumed;

        // Anything that changes what's on screen (input we act on, egui asking
        // for a repaint, a resize, a dropped file) needs a fresh frame; the
        // loop is otherwise allowed to sleep once the image has converged.
        let mut wants_redraw = response.repaint;

        match &event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.focused = *focused;
                wants_redraw |= self.focused;
            }
            WindowEvent::Resized(size) => {
                self.controller.resize(*size);
                wants_redraw = true;
            }
            WindowEvent::RedrawRequested => {
                // A previous frame's GPU work is still draining; skip so we
                // don't queue a second one behind it. The completion callback
                // re-requests a redraw once it lands.
                if self.in_flight.load(Ordering::SeqCst) {
                    return;
                }

                self.fps.tick();

                let outcome =
                    renderer.render(&self.gpu, &window, &mut self.controller, self.fps.seconds());

                if outcome.accumulating || self.dirty {
                    // Another frame is wanted - re-arm once the GPU drains, so
                    // exactly one render is ever in flight.
                    self.dirty = false;
                    self.in_flight.store(true, Ordering::SeqCst);
                    let window = Arc::clone(&window);
                    let in_flight = Arc::clone(&self.in_flight);
                    self.gpu.queue().on_submitted_work_done(move || {
                        in_flight.store(false, Ordering::SeqCst);
                        window.request_redraw();
                    });
                } else if outcome.repaint_after < Duration::MAX {
                    let at = Instant::now() + outcome.repaint_after;
                    self.next_repaint =
                        Some(self.next_repaint.map_or(at, |existing| existing.min(at)));
                }

                self.gpu.wait();
            }
            WindowEvent::DroppedFile(path) => {
                FileStage::load_path(path);
                wants_redraw = true;
            }
            _ => (),
        }

        if let Some(vibrant_event) = vibrant_event(event) {
            if self.controller.hovered() | !consumed {
                self.controller.event(vibrant_event);
                wants_redraw = true;
            }
        }

        if wants_redraw {
            self.dirty = true;
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut attributes = window::Window::default_attributes();
        attributes = attributes
            .with_title("VIBRANT")
            .with_visible(false)
            .with_maximized(true);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use web_sys::{window, HtmlCanvasElement};
            use winit::platform::web::WindowAttributesExtWebSys;

            let canvas = window()
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

        window.set_visible(true);
        window.request_redraw();

        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.event(event_loop, event);
    }

    /// Idle policy: sleep until the OS wakes us, unless egui asked for a timed
    /// repaint (`next_repaint`), in which case wait exactly that long and then
    /// request one. Active accumulation doesn't rely on this - it re-arms via
    /// `on_submitted_work_done` in `RedrawRequested`.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.next_repaint {
            Some(at) if Instant::now() >= at => {
                self.next_repaint = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Some(at) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
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
                    state: ElementState::Pressed,
                    ..
                },
            is_synthetic: _,
        } => keycode(code).map(Event::KeyPressed),
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: ElementState::Released,
                    ..
                },
            ..
        } => keycode(code).map(Event::KeyReleased),
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

    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
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

impl<const N: usize> Default for Fps<N> {
    fn default() -> Self {
        Self::new()
    }
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
