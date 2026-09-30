use std::{any::type_name, cell::Cell};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, CommandEncoder,
    RenderPassDescriptor, RenderPipeline, ShaderStages, TextureSampleType, TextureViewDimension,
};

use crate::{
    asset::{
        environment::Environment, hdri::HdriBuffer, radiance::GaussianRadianceBuffer,
        volume::PhysicalVolume, Asset,
    },
    controller::Controller,
    gpu::Gpu,
    renderer::lighting::VMM_SIZE_OPTIONS,
    surface::{color::ColorBuffer, Frame},
};

pub struct GaussianVolumeRenderer {
    // One precompiled trace pipeline per VMM_SIZE_OPTIONS entry.
    variants: [RenderPipeline; 3],
    // Group 3: HDRI material settings + the combined-mode line-depth target.
    // Built per dispatch since the line-depth view lives on the (resizable) Frame.
    material_layout: BindGroupLayout,
    active: Cell<usize>,
}

impl GaussianVolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let trace_src = include_str!("trace.wgsl");

        let material_layout = Self::material_layout(gpu);

        let variants = VMM_SIZE_OPTIONS.map(|vmm_size| {
            gpu.quad(
                "GaussianVolumeTrace",
                &gpu.pipeline_layout(&[
                    &GaussianRadianceBuffer::layout(gpu),
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &material_layout,
                ]),
                ColorBuffer::target_premultiplied(),
                &gpu.shader(&vmm_template(trace_src, vmm_size)),
            )
        });

        let active = Cell::new(
            VMM_SIZE_OPTIONS
                .iter()
                .position(|&size| size == 32)
                .unwrap(),
        );

        Self {
            variants,
            material_layout,
            active,
        }
    }

    /// Group 3 for `trace.wgsl`: the `HdriSettings` uniform (binding 0) plus the
    /// line-depth target (binding 1). The environment map itself is folded into
    /// the radiance cascade, so the tracer doesn't sample it here.
    fn material_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
    }

    fn set_vmm_size(&self, vmm_size: u32) {
        if let Some(index) = VMM_SIZE_OPTIONS.iter().position(|&size| size == vmm_size) {
            self.active.set(index);
        }
    }

    pub fn dispatch(
        &self,
        gpu: &Gpu,
        cmd: &mut CommandEncoder,
        asset: &Asset,
        controller: &Controller,
        frame: &Frame,
    ) {
        let (Some(volume), Some(radiance)) = (&asset.physical_volume, &asset.radiance) else {
            return;
        };

        self.set_vmm_size(controller.radiance().lobes());

        let material = self.material_binding(gpu, &asset.hdri, frame);

        let mut pass = cmd.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(frame.color().attachment())],
            ..Default::default()
        });

        let viewport = controller.viewport();
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
        pass.set_bind_group(2, asset.environment.binding(), &[]);
        pass.set_bind_group(3, &material, &[]);
        pass.draw(0..4, 0..1);
    }

    fn material_binding(&self, gpu: &Gpu, hdri: &HdriBuffer, frame: &Frame) -> BindGroup {
        gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &self.material_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: hdri.settings_buffer().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(frame.line_depth().view()),
                },
            ],
        })
    }
}

fn vmm_template(src: &str, vmm_size: u32) -> String {
    src.replace("#VMM_SIZE", &vmm_size.to_string())
}
