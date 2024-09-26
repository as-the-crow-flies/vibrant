use std::sync::Arc;

use wgpu::{
    CompositeAlphaMode, PresentMode, SurfaceCapabilities, SurfaceConfiguration, SurfaceTexture,
    TextureFormat, TextureUsages,
};
use winit::window::Window;

use super::gpu::Gpu;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
}

impl Surface {
    pub const VIEW_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

    pub fn new(gpu: &Gpu, window: Arc<Window>, width: u32, height: u32) -> Self {
        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let format = Self::choose_format(surface.get_capabilities(gpu.adapter()));

        let surface = Self { surface, format };

        surface.configure(gpu, width, height);

        surface
    }

    pub fn configure(&self, gpu: &Gpu, width: u32, height: u32) {
        self.surface.configure(
            gpu.device(),
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width,
                height,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 3,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![Self::VIEW_FORMAT],
            },
        );
    }

    pub fn get_current_texture(&self) -> SurfaceTexture {
        self.surface
            .get_current_texture()
            .expect("Could not obtain surface texture")
    }

    fn choose_format(capabilities: SurfaceCapabilities) -> TextureFormat {
        if capabilities
            .formats
            .contains(&TextureFormat::Bgra8UnormSrgb)
        {
            return TextureFormat::Bgra8UnormSrgb;
        }
        if capabilities.formats.contains(&TextureFormat::Bgra8Unorm) {
            return TextureFormat::Bgra8Unorm;
        }

        panic!("Surface Format BgraUnorm(Srgb) is not available")
    }
}
