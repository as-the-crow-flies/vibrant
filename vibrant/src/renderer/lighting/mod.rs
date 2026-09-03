use std::cell::Cell;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{hdri::HdriBuffer, radiance::GaussianRadianceBuffer, volume::PhysicalVolume},
    gpu::Gpu,
    renderer::environment::Environment,
};

// Selectable lobe counts -- see RenderingWidget::lobes. Each entry gets its own
// fully precompiled set of pipelines (below) so switching at runtime is just
// picking which one to dispatch, no shader recompilation involved.
pub const VMM_SIZE_OPTIONS: [u32; 3] = [8, 16, 32];

struct LightingVariant {
    hdri: ComputePipeline,
    cascade: Vec<ComputePipeline>,
}

pub struct LightingRenderer {
    variants: [LightingVariant; 3],
    active: Cell<usize>,
}

impl LightingRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let common = include_str!("common.wgsl");

        let hdri_src = include_str!("hdri.wgsl").to_string() + common;
        let cascade_src = include_str!("cascade.wgsl").to_string() + common;

        let variants = VMM_SIZE_OPTIONS.map(|vmm_size| LightingVariant {
            hdri: gpu.compute(
                "GaussianVolumeHdri",
                &gpu.pipeline_layout(&[
                    &GaussianRadianceBuffer::layout_hdri(gpu),
                    &PhysicalVolume::layout_read(gpu),
                    &Environment::layout(gpu),
                    &HdriBuffer::layout(gpu),
                ]),
                &gpu.shader(&vmm_template(&hdri_src, vmm_size)),
            ),
            cascade: (0..GaussianRadianceBuffer::LEVELS)
                .map(|cascade| {
                    gpu.compute(
                        "GaussianVolumeCascade",
                        &gpu.pipeline_layout(&[
                            &GaussianRadianceBuffer::layout_mipmap(gpu),
                            &PhysicalVolume::layout_read(gpu),
                            &Environment::layout(gpu),
                            &HdriBuffer::layout(gpu),
                        ]),
                        &gpu.shader(&vmm_template(
                            &cascade_template(&cascade_src, cascade),
                            vmm_size,
                        )),
                    )
                })
                .collect(),
        });

        // Defaults to the 32-lobe variant (index of 32 in VMM_SIZE_OPTIONS),
        // matching the previous hardcoded behavior for callers -- like the
        // cascade benchmark -- that dispatch hdri/cascade directly and never
        // call dispatch() to select a variant.
        let active = Cell::new(VMM_SIZE_OPTIONS.iter().position(|&size| size == 32).unwrap());

        Self { variants, active }
    }

    fn variant(&self) -> &LightingVariant {
        &self.variants[self.active.get()]
    }

    fn set_vmm_size(&self, vmm_size: u32) {
        if let Some(index) = VMM_SIZE_OPTIONS.iter().position(|&size| size == vmm_size) {
            self.active.set(index);
        }
    }

    /// Build one cascade (`hdri` fit + all levels) for an explicit
    /// `PhysicalVolume` / `GaussianRadianceBuffer` pair. The renderer calls this
    /// once per active PV/radiance pair (volume, lines, or both).
    pub fn dispatch_for(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
        lobes: u32,
    ) {
        self.set_vmm_size(lobes);

        self.hdri(cmd, environment, hdri, radiance, volume);
        self.radiance(cmd, environment, hdri, radiance, volume);
    }

    pub fn hdri(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("GaussianVolumeHdri"),
            ..Default::default()
        });

        pass.set_pipeline(&self.variant().hdri);
        pass.set_bind_group(0, radiance.binding_hdri(), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        pass.dispatch_workgroups(1, 1, 1);
    }

    pub fn radiance(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
    ) {
        for cascade in (0..GaussianRadianceBuffer::LEVELS as usize).rev() {
            self.cascade(cmd, environment, hdri, radiance, volume, cascade);
        }
    }

    pub fn cascade(
        &self,
        cmd: &mut CommandEncoder,
        environment: &Environment,
        hdri: &HdriBuffer,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
        cascade: usize,
    ) {
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some(&format!("Cascade {}", cascade)),
            ..Default::default()
        });

        pass.set_pipeline(&self.variant().cascade[cascade]);
        pass.set_bind_group(0, radiance.binding_mipmap(cascade), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        let probes = radiance.probes(cascade);
        pass.dispatch_workgroups(probes.x, probes.y, probes.z);
    }
}

fn cascade_template(src: &str, cascade: u32) -> String {
    src.replace("#CASCADE", &cascade.to_string())
        .to_string()
        .replace(
            "#WORKGROUP",
            &match cascade {
                5 => 1024,
                4 => 256,
                3 => 64,
                _ => 32,
            }
            .to_string(),
        )
        .replace(
            "#SUBGROUPS",
            &match cascade {
                5 => 32,
                4 => 8,
                3 => 2,
                _ => 1,
            }
            .to_string(),
        )
        .replace(
            "#SAMPLES",
            &match cascade {
                5 => 4096,
                4 => 1024,
                3 => 256,
                _ => 128,
            }
            .to_string(),
        )
        .to_string()
}

fn vmm_template(src: &str, vmm_size: u32) -> String {
    src.replace("#VMM_SIZE", &vmm_size.to_string())
}
