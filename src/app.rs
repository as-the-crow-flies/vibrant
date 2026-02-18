use std::path::PathBuf;
use std::sync::Arc;
use vibrant::controller::event::{Key, MouseButton};
use vibrant::file::FileStage;
use vibrant::gpu::Gpu;
use vibrant::Vec2;
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use crate::video::VideoEncoder;
use pollster::FutureExt;

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

/// The mode the application should run in.
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Normal interactive GUI mode
    Interactive,
    /// Render a single frame, save as PNG, then exit
    Screenshot { output: PathBuf },
    /// Render frames over a duration, pipe to ffmpeg, then exit
    Video {
        output: PathBuf,
        fps: u32,
        duration: u32,
    },
}

/// Configuration passed from CLI (or defaults for WASM).
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub input: Vec<PathBuf>,
    pub mode: AppMode,
    pub auto_rotate: bool,
    pub rotate_speed: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            input: Vec::new(),
            mode: AppMode::Interactive,
            auto_rotate: false,
            rotate_speed: 10.0,
        }
    }
}

struct App {
    gpu: Gpu,
    window: Option<Arc<window::Window>>,
    renderer: Option<Renderer>,
    controller: Controller,
    focused: bool,
    fps: Fps<8>,
    config: AppConfig,
    frames_rendered: u32,
    screenshot_triggered: bool,
    #[cfg(not(target_arch = "wasm32"))]
    video_encoder: Option<VideoEncoder>,
    video_frames_written: u32,
    video_total_frames: u32,
}

impl App {
    fn new(gpu: Gpu, config: AppConfig) -> Self {
        let video_total_frames = if let AppMode::Video { fps, duration, .. } = &config.mode {
            fps * duration
        } else {
            0
        };
        Self {
            gpu,
            window: None,
            renderer: None,
            controller: Controller::new(),
            focused: true,
            fps: Fps::new(),
            config,
            frames_rendered: 0,
            screenshot_triggered: false,
            #[cfg(not(target_arch = "wasm32"))]
            video_encoder: None,
            video_frames_written: 0,
            video_total_frames,
        }
    }

    fn event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let window = self.window.as_ref().expect("Window");
        let renderer = self.renderer.as_mut().expect("Renderer");

        let consumed_by_egui = renderer.egui().on_window_event(window, &event).consumed;

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

                // Use fixed dt for video mode, real-time dt otherwise
                #[cfg(not(target_arch = "wasm32"))]
                let dt = if let AppMode::Video { fps, .. } = &self.config.mode {
                    if self.video_encoder.is_some() {
                        1.0 / *fps as f32
                    } else {
                        self.fps.seconds()
                    }
                } else {
                    self.fps.seconds()
                };
                #[cfg(target_arch = "wasm32")]
                let dt = self.fps.seconds();

                renderer.render(&self.gpu, window, &mut self.controller, dt);

                self.frames_rendered += 1;

                // Screenshot mode: wait for assets to load and a few frames for GPU
                // pipeline warmup, then trigger a save and exit.
                if let AppMode::Screenshot { ref output } = self.config.mode {
                    if renderer.has_assets()
                        && self.frames_rendered >= 3
                        && !self.screenshot_triggered
                    {
                        self.screenshot_triggered = true;
                        FileStage::save_path(output.clone());
                        // Need one more frame to execute the save in render()
                        self.request_redraw();
                        return;
                    }
                    if self.screenshot_triggered && self.frames_rendered >= 4 {
                        log::info!("Screenshot saved, exiting.");
                        event_loop.exit();
                        return;
                    }
                }

                // Video mode: once assets are loaded, start encoding frames.
                #[cfg(not(target_arch = "wasm32"))]
                if let AppMode::Video {
                    ref output,
                    fps,
                    duration,
                } = self.config.mode
                {
                    if renderer.has_assets() && self.frames_rendered >= 3 {
                        // Initialize encoder on first video frame
                        if self.video_encoder.is_none() {
                            // Force auto-rotate: full 360° over the video duration
                            let rotate_speed = 360.0 / duration as f32;
                            self.controller.settings_mut().auto_rotate = true;
                            self.controller.settings_mut().auto_rotate_speed = rotate_speed;

                            match renderer.read_frame(&self.gpu).block_on() {
                                Ok((_, w, h)) => {
                                    match VideoEncoder::new(output, w, h, fps) {
                                        Ok(encoder) => {
                                            log::info!(
                                                "Video recording started: {}x{} @ {} fps, {} seconds ({} frames)",
                                                w, h, fps, duration, self.video_total_frames
                                            );
                                            self.video_encoder = Some(encoder);
                                        }
                                        Err(e) => {
                                            log::error!("Failed to start video encoder: {}", e);
                                            event_loop.exit();
                                            return;
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to read frame for video init: {}", e);
                                    event_loop.exit();
                                    return;
                                }
                            }
                        }

                        if self.video_frames_written < self.video_total_frames {
                            // Read back the frame that was just rendered above
                            match renderer.read_frame(&self.gpu).block_on() {
                                Ok((data, _, _)) => {
                                    if let Some(encoder) = &mut self.video_encoder {
                                        if let Err(e) = encoder.write_frame(&data) {
                                            log::error!("Failed to write video frame: {}", e);
                                            // Take the encoder to finish/drop it gracefully
                                            if let Some(enc) = self.video_encoder.take() {
                                                let _ = enc.finish();
                                            }
                                            event_loop.exit();
                                            return;
                                        }
                                    }
                                    self.video_frames_written += 1;
                                }
                                Err(e) => {
                                    log::error!("Failed to read frame for video: {}", e);
                                    if let Some(enc) = self.video_encoder.take() {
                                        let _ = enc.finish();
                                    }
                                    event_loop.exit();
                                    return;
                                }
                            }
                        } else {
                            // Done: finish encoding and exit
                            if let Some(encoder) = self.video_encoder.take() {
                                if let Err(e) = encoder.finish() {
                                    log::error!("Failed to finalize video: {}", e);
                                }
                            }
                            log::info!("Video saved, exiting.");
                            event_loop.exit();
                            return;
                        }
                    }
                }

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

        self.window = Some(window);
        self.renderer = Some(renderer);

        // Apply CLI settings
        if self.config.auto_rotate {
            self.controller.settings_mut().auto_rotate = true;
            self.controller.settings_mut().auto_rotate_speed = self.config.rotate_speed;
        }

        // Load input files specified via CLI
        #[cfg(not(target_arch = "wasm32"))]
        for path in &self.config.input {
            if let Err(e) = FileStage::load_path(path.clone()) {
                log::error!("Failed to load {:?}: {}", path, e);
            }
        }
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

pub async fn run(config: AppConfig) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(Gpu::new().await, config);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert!(config.input.is_empty());
        assert_eq!(config.mode, AppMode::Interactive);
        assert!(!config.auto_rotate);
        assert_eq!(config.rotate_speed, 10.0);
    }
}
