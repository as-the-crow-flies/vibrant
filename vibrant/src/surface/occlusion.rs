use std::any::type_name;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, FilterMode,
};

use crate::{
    asset::scalar::{R32Float, ScalarTexture3D},
    gpu::Gpu,
};

pub struct OcclusionBuffer {
    ambient: ScalarTexture3D<R32Float>,
    directional: ScalarTexture3D<R32Float>,
    binding: BindGroup,
}

impl OcclusionBuffer {
    pub fn new(gpu: &Gpu, resolution: u32) -> Self {
        let ambient = ScalarTexture3D::<R32Float>::new(gpu, resolution, FilterMode::Linear);
        let directional = ScalarTexture3D::<R32Float>::new(gpu, resolution, FilterMode::Linear);

        let binding = gpu.device().create_bind_group(&BindGroupDescriptor {
            label: Some(type_name::<Self>()),
            layout: &Self::layout(gpu),
            entries: &[ambient.binding_entries(0), directional.binding_entries(2)].concat(),
        });

        Self {
            ambient,
            directional,
            binding,
        }
    }

    pub fn ambient(&self) -> &ScalarTexture3D<R32Float> {
        &self.ambient
    }

    pub fn directional(&self) -> &ScalarTexture3D<R32Float> {
        &self.directional
    }

    pub fn binding(&self) -> &BindGroup {
        &self.binding
    }

    pub fn layout(gpu: &Gpu) -> BindGroupLayout {
        gpu.device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(type_name::<Self>()),
                entries: &[
                    ScalarTexture3D::<R32Float>::layout_entries(0),
                    ScalarTexture3D::<R32Float>::layout_entries(2),
                ]
                .concat(),
            })
    }
}
