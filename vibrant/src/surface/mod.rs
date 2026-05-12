pub mod color;
pub mod culling;
pub mod occlusion;
pub mod occupancy;

use std::any::type_name;

use color::ColorBuffer;
use occlusion::OcclusionBuffer;
use occupancy::OccupancyBuffer;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, CommandEncoder,
    CompositeAlphaMode, CurrentSurfaceTexture, Extent3d, Origin3d, PresentMode,
    SurfaceConfiguration, SurfaceTarget, SurfaceTexture, TexelCopyTextureInfo, TextureAspect,
    TextureFormat, TextureUsages,
};

use crate::{
    asset::texture::{MipTexture3D, R32Float, R32Uint},
    controller::settings::Settings,
    surface::culling::CullingBuffer,
};

use super::gpu::Gpu;

pub struct Frame {
    color: ColorBuffer,
    post: ColorBuffer,
    occupancy: OccupancyBuffer,
    occlusion: OcclusionBuffer,
    culling: CullingBuffer,
    binding: BindGroup,
}

impl Frame {
    pub fn new(gpu: &Gpu, settings: &Settings) -> Self {
        let color = ColorBuffer::new(gpu, settings.width, settings.height);
        let post = ColorBuffer::new(gpu, settings.width, settings.height);

        let occupancy = OccupancyBuffer::new(gpu, settings.volume);
        let occlusion = OcclusionBuffer::new(gpu, settings.volume);
        let culling = CullingBuffer::new(gpu, settings.volume, settings.index_size);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[
                occupancy.pyramid().binding_entries(0),
                occupancy.count().binding_entries(2),
                occlusion.ambient().binding_entries(4),
                occlusion.directional().binding_entries(6),
            ]
            .concat(),
        });

        Self {
            color,
            post,
            occupancy,
            occlusion,
            culling,
            binding,
        }
    }

    pub fn color(&self) -> &ColorBuffer {
        &self.color
    }

    pub fn post(&self) -> &ColorBuffer {
        &self.post
    }

    pub fn occupancy(&self) -> &OccupancyBuffer {
        &self.occupancy
    }

    pub fn occlusion(&self) -> &OcclusionBuffer {
        &self.occlusion
    }

    pub fn culling(&self) -> &CullingBuffer {
        &self.culling
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    MipTexture3D::<R32Float>::layout_entries(0), // Occupancy - Density
                    MipTexture3D::<R32Uint>::layout_entries(2),  // Occupancy - Count
                    MipTexture3D::<R32Float>::layout_entries(4), // Occlusion - Ambient
                    MipTexture3D::<R32Float>::layout_entries(6), // Occlusion - Directional
                ]
                .concat(),
            })
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    frame: Option<Frame>,
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
        }
    }

    pub fn maybe_resize(&mut self, gpu: &Gpu, settings: &mut Settings) {
        if let Some(frame) = &self.frame {
            // Update Required Index Size

            let required_index_size = frame.culling().get_required_index_size(gpu);
            if required_index_size > settings.index_size {
                settings.index_size = required_index_size.next_power_of_two()
            }

            if settings.width == frame.color().width()
                && settings.height == frame.color().height()
                && settings.volume == frame.occupancy().resolution()
                && settings.index_size == frame.culling().fragment_list_size()
            {
                self.changed = false;
                return;
            }
        }

        self.frame.take();
        self.frame = Some(Frame::new(gpu, &settings));

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
            cmd.copy_texture_to_texture(
                TexelCopyTextureInfo {
                    texture: frame.post().texture(),
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                TexelCopyTextureInfo {
                    texture: &surface.texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: surface.texture.width(),
                    height: surface.texture.height(),
                    depth_or_array_layers: 1,
                },
            );

            gpu.submit(cmd);
            surface.present();
        }
    }

    fn config(width: u32, height: u32) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
            format: Self::FORMAT,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![Self::FORMAT],
        }
    }

    pub fn frame(&self) -> &Option<Frame> {
        &self.frame
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
