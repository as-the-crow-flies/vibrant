use std::{any::type_name, sync::Arc};

use wgpu::{
    CompositeAlphaMode, Extent3d, PresentMode, SurfaceCapabilities, SurfaceConfiguration,
    SurfaceTexture, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

use super::gpu::Gpu;

pub struct Frame {
    surface: SurfaceTexture,
    color: TextureView,
    depth: TextureView,
}

impl Frame {
    pub fn new(surface: SurfaceTexture, depth: &Texture) -> Self {
        Self {
            color: surface.texture.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::VIEW_FORMAT),
                ..Default::default()
            }),
            depth: depth.create_view(&TextureViewDescriptor {
                label: Some(type_name::<Self>()),
                format: Some(Surface::DEPTH_FORMAT),
                ..Default::default()
            }),
            surface,
        }
    }

    pub fn color(&self) -> &TextureView {
        &self.color
    }

    pub fn depth(&self) -> &TextureView {
        &self.depth
    }

    pub fn present(self) {
        self.surface.present();
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    format: TextureFormat,
    depth: Texture, // TODO: Depth Texture per Swap Chain Texture!?
}

impl Surface {
    pub const VIEW_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn new(gpu: &Gpu, window: Arc<Window>) -> Self {
        let size = window.inner_size().max(PhysicalSize::new(1, 1));

        let surface = gpu
            .instance()
            .create_surface(window)
            .expect("Could not create surface");

        let format = Self::choose_format(surface.get_capabilities(gpu.adapter()));

        let mut surface = Self {
            surface,
            format,
            depth: Self::create_depth_texture(gpu, size.width, size.height),
        };

        surface.resize(gpu, size.width, size.height);

        surface
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.surface.configure(
            gpu.device(),
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width,
                height,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![Self::VIEW_FORMAT],
            },
        );

        self.depth = Self::create_depth_texture(gpu, width, height);
    }

    pub fn frame(&self) -> Frame {
        Frame::new(
            self.surface
                .get_current_texture()
                .expect("Could not obtain surface texture"),
            &self.depth,
        )
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

    fn create_depth_texture(gpu: &Gpu, width: u32, height: u32) -> Texture {
        gpu.device().create_texture(&TextureDescriptor {
            label: Some(type_name::<Self>()),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[Self::DEPTH_FORMAT],
        })
    }
}
