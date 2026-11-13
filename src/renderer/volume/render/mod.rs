use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, CommandEncoder,
    RenderPassDescriptor, RenderPipeline, ShaderStages, TextureSampleType, TextureViewDimension,
};

use crate::{
    asset::{
        environment::Environment,
        hdri::HdriBuffer,
        radiance::GaussianRadianceBuffer,
        texture::{MipTexture3D, R32Float},
        volume::PhysicalVolume,
        Asset,
    },
    controller::Controller,
    gpu::{Gpu, Lazy},
    surface::{color::ColorBuffer, Frame},
};

pub struct GaussianVolumeRenderer {
    pipeline: Lazy<RenderPipeline>,
    /// Per microfacet and mirror Fresnel.
    traced: [Lazy<RenderPipeline>; 2],
    // Group 3: HDRI material settings + the combined-mode line-depth target.
    // Built per dispatch since the line-depth view lives on the (resizable) Frame.
    material_layout: BindGroupLayout,
    classified_layout: BindGroupLayout,
}

impl GaussianVolumeRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let material_layout = Self::material_layout(gpu);
        let classified_layout = PhysicalVolume::layout_classified(gpu);

        let layout = gpu.pipeline_layout(&[
            &GaussianRadianceBuffer::layout(gpu),
            &classified_layout,
            &Environment::layout(gpu),
            &material_layout,
        ]);

        let pipeline = |label: &'static str, traced: bool, mirror: bool| {
            let layout = layout.clone();
            gpu.lazy(label, move |gpu| {
                gpu.quad(
                    label,
                    &layout,
                    ColorBuffer::target_premultiplied(),
                    &gpu.shader_with(
                        "renderer/volume/render/trace.wgsl",
                        &[("TRACED", &traced), ("MIRROR_FRESNEL", &mirror)],
                    ),
                )
            })
        };

        Self {
            pipeline: pipeline("GaussianVolumeTrace", false, false),
            traced: [
                pipeline("GaussianVolumeTrace::Traced", true, false),
                pipeline("GaussianVolumeTrace::TracedMirror", true, true),
            ],
            material_layout,
            classified_layout,
        }
    }

    /// Group 3 for `trace.wgsl`: the `HdriSettings` uniform, the line-depth
    /// target, and for the ray traced mode the environment map and line density.
    fn material_layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    vec![
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
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: false },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                    MipTexture3D::<R32Float>::layout_entries(3),
                ]
                .concat(),
            })
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

        let classified = volume.binding_classified(
            gpu,
            &self.classified_layout,
            &asset.visible_volumes(),
            &asset.masks,
        );

        let material = self.material_binding(gpu, &asset.hdri, volume, frame);

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

        let settings = controller.settings();
        if settings.ray_traced {
            pass.set_pipeline(&self.traced[settings.mirror_fresnel as usize]);
        } else {
            pass.set_pipeline(&self.pipeline);
        }

        pass.set_bind_group(0, radiance.binding(), &[]);
        pass.set_bind_group(1, &classified, &[]);
        pass.set_bind_group(2, asset.environment.binding(), &[]);
        pass.set_bind_group(3, &material, &[]);
        pass.draw(0..4, 0..1);
    }

    fn material_binding(
        &self,
        gpu: &Gpu,
        hdri: &HdriBuffer,
        volume: &PhysicalVolume,
        frame: &Frame,
    ) -> BindGroup {
        gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &self.material_layout,
            entries: &[
                vec![
                    BindGroupEntry {
                        binding: 0,
                        resource: hdri.settings_buffer().as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(frame.line_depth().view()),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(hdri.map()),
                    },
                ],
                volume.line_extinction().binding_entries(3),
            ]
            .concat(),
        })
    }
}
