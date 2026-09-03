use std::cell::Cell;

use egui::Rect;
use wgpu::{CommandEncoder, RenderPassDescriptor, RenderPipeline};

use crate::{
    asset::{hdri::HdriBuffer, radiance::GaussianRadianceBuffer, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::{environment::Environment, lighting::VMM_SIZE_OPTIONS},
    surface::{color::ColorBuffer, Frame},
};

pub struct GaussianVolumeRenderer {
    // One precompiled trace pipeline per VMM_SIZE_OPTIONS entry.
    variants: [RenderPipeline; 3],
    active: Cell<usize>,
}

impl GaussianVolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let trace_src = include_str!("trace.wgsl");

        let variants = VMM_SIZE_OPTIONS.map(|vmm_size| {
            gpu.quad(
                "GaussianVolumeTrace",
                &gpu.pipeline_layout(&[
                    &GaussianRadianceBuffer::layout(gpu),
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                ColorBuffer::target(),
                &gpu.shader(&vmm_template(trace_src, vmm_size)),
            )
        });

        let active = Cell::new(VMM_SIZE_OPTIONS.iter().position(|&size| size == 32).unwrap());

        Self { variants, active }
    }

    fn set_vmm_size(&self, vmm_size: u32) {
        if let Some(index) = VMM_SIZE_OPTIONS.iter().position(|&size| size == vmm_size) {
            self.active.set(index);
        }
    }

    pub fn dispatch(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
        viewport: Rect,
        vmm_size: u32,
    ) {
        self.set_vmm_size(vmm_size);

        self.trace(cmd, environment, hdri, frame, radiance, volume, viewport);
    }

    fn trace(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        frame: &Frame,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
        viewport: Rect,
    ) {
        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment())],
            ..Default::default()
        });

        pass.set_viewport(
            viewport.min.x,
            viewport.min.y,
            viewport.width(),
            viewport.height(),
            0.0,
            1.0,
        );

        pass.set_pipeline(&self.variants[self.active.get()]);

        pass.set_bind_group(0, radiance.binding(), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);
        pass.draw(0..4, 0..1);
    }
}

fn vmm_template(src: &str, vmm_size: u32) -> String {
    src.replace("#VMM_SIZE", &vmm_size.to_string())
}
