pub mod color;

use color::ColorBuffer;
use wgpu::{
    ColorTargetState, ColorWrites, CommandEncoder, CompositeAlphaMode, CurrentSurfaceTexture,
    PresentMode, SurfaceConfiguration, SurfaceTarget, SurfaceTexture, TextureFormat, TextureUsages,
};

use crate::{controller::settings::Settings, renderer::util::copy::CopyPipeline};

use super::gpu::Gpu;

pub struct Frame {
    color: ColorBuffer,
}

impl Frame {
    pub fn new(gpu: &Gpu, settings: &Settings) -> Self {
        Self {
            color: ColorBuffer::new(gpu, settings.width, settings.height),
        }
    }

    pub fn color(&self) -> &ColorBuffer {
        &self.color
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    frame: Option<Frame>,
    copy: CopyPipeline,
    changed: bool,
}

impl Surface {
    const FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

    pub fn new(gpu: &Gpu, window: impl Into<SurfaceTarget<'static>>) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        surface.configure(gpu.device(), &Self::config(1, 1));

        Self {
            surface,
            frame: None,
            changed: true,
            copy: CopyPipeline::new(gpu),
        }
    }

    pub fn maybe_resize(&mut self, gpu: &Gpu, settings: &Settings) {
        if let Some(frame) = &self.frame {
            if settings.width == frame.color().width() && settings.height == frame.color().height()
            {
                self.changed = false;
                return;
            }
        }

        self.frame.take();
        self.frame = Some(Frame::new(gpu, settings));

        self.surface
            .configure(gpu.device(), &Self::config(settings.width, settings.height));

        self.changed = true;
    }

    fn get_current_texture(&self) -> Option<SurfaceTexture> {
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => Some(texture),
            CurrentSurfaceTexture::Suboptimal(texture) => Some(texture),
            _ => None,
        }
    }

    pub fn present(&self, gpu: &Gpu, mut cmd: CommandEncoder) {
        if let (Some(frame), Some(surface)) = (&self.frame, self.get_current_texture()) {
            self.copy
                .dispatch(&mut cmd, frame.color(), &surface.texture);

            gpu.submit(cmd);
            gpu.queue().present(surface);
        }
    }

    fn config(width: u32, height: u32) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: Self::FORMAT,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![Self::FORMAT],
            color_space: wgpu::SurfaceColorSpace::Auto,
        }
    }

    pub fn target() -> ColorTargetState {
        ColorTargetState {
            format: Self::FORMAT,
            blend: None,
            write_mask: ColorWrites::all(),
        }
    }

    pub fn frame(&self) -> &Option<Frame> {
        &self.frame
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
