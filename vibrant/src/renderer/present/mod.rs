use std::any::type_name;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    Color, ColorTargetState, ColorWrites, CommandEncoder, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, ShaderStages, StoreOp,
    TextureFormat, TextureView,
};

use crate::{gpu::Gpu, surface::color::ColorBuffer};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PresentUniform {
    headroom: f32,
    exposure: f32,
    _pad: [f32; 2],
}

/// How the swapchain wants its pixels, picked from surface capabilities.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presentation {
    /// `Bgra8Unorm` + `Auto`: tone map, sRGB-encoded output.
    Sdr,
    /// `Rgba16Float` + `ExtendedSrgbLinear` (native HDR): linear output.
    HdrLinear,
    /// `Rgba16Float` + `ExtendedSrgb` (web HDR): sRGB-encoded, extended past 1.0.
    HdrEncoded,
}

/// Turns the linear HDR scene buffer into the swapchain image: tone map once
/// (targeting the display headroom in the HDR variants) and composite the egui
/// overlay on top. One pipeline is baked per [`Presentation`]; the per-frame
/// headroom/exposure ride in a small uniform.
pub struct PresentPipeline {
    sdr: RenderPipeline,
    hdr_linear: RenderPipeline,
    hdr_encoded: RenderPipeline,
    /// SDR variant targeting [`ColorBuffer::LDR_FORMAT`] for screenshot readback
    /// (the swapchain is usually `Bgra8Unorm`, but `Gpu::save` wants RGBA bytes).
    export: RenderPipeline,
    uniform: Buffer,
    binding: BindGroup,
}

impl PresentPipeline {
    pub fn new(gpu: &Gpu, sdr_format: TextureFormat, hdr_format: TextureFormat) -> Self {
        let uniform_layout = Self::uniform_layout(gpu);

        let layout = gpu.pipeline_layout(&[
            &ColorBuffer::layout(gpu),
            &ColorBuffer::layout(gpu),
            &uniform_layout,
        ]);

        let flag = |value: bool| if value { "true" } else { "false" };

        let build = |hdr: bool, linear_output: bool, export: bool, format: TextureFormat| {
            let source = include_str!("present.wgsl")
                .replace("#HDR", flag(hdr))
                .replace("#LINEAR_OUTPUT", flag(linear_output))
                .replace("#EXPORT", flag(export));

            gpu.quad(
                type_name::<Self>(),
                &layout,
                ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::all(),
                },
                &gpu.shader(&source),
            )
        };

        let uniform = gpu.device().create_buffer(&BufferDescriptor {
            label: Some(type_name::<Self>()),
            size: std::mem::size_of::<PresentUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &uniform_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        Self {
            sdr: build(false, false, false, sdr_format),
            hdr_linear: build(true, true, false, hdr_format),
            hdr_encoded: build(true, false, false, hdr_format),
            export: build(false, false, true, ColorBuffer::LDR_FORMAT),
            uniform,
            binding,
        }
    }

    fn uniform_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }

    /// On-screen present: `scene` + `overlay` -> swapchain `target`.
    pub fn dispatch(
        &self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        scene: &ColorBuffer,
        overlay: &ColorBuffer,
        target: &TextureView,
        presentation: Presentation,
        headroom: f32,
    ) {
        let pipeline = match presentation {
            Presentation::Sdr => &self.sdr,
            Presentation::HdrLinear => &self.hdr_linear,
            Presentation::HdrEncoded => &self.hdr_encoded,
        };
        self.run(gpu, cmd, pipeline, scene, overlay, target, headroom);
    }

    /// SDR present into an `Rgba8Unorm` `target` for screenshot readback.
    pub fn export(
        &self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        scene: &ColorBuffer,
        overlay: &ColorBuffer,
        target: &TextureView,
    ) {
        self.run(gpu, cmd, &self.export, scene, overlay, target, 1.0);
    }

    fn run(
        &self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        pipeline: &RenderPipeline,
        scene: &ColorBuffer,
        overlay: &ColorBuffer,
        target: &TextureView,
        headroom: f32,
    ) {
        gpu.queue().write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&PresentUniform {
                headroom,
                exposure: 1.0,
                _pad: [0.0; 2],
            }),
        );

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some(type_name::<Self>()),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, scene.binding(), &[]);
        pass.set_bind_group(1, overlay.binding(), &[]);
        pass.set_bind_group(2, &self.binding, &[]);
        pass.draw(0..4, 0..1);
    }
}
