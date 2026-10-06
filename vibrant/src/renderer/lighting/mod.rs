use std::cell::Cell;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        environment::Environment, hdri::HdriBuffer, radiance::GaussianRadianceBuffer,
        volume::PhysicalVolume, Asset,
    },
    controller::Controller,
    gpu::Gpu,
    renderer::wgsl::em::{EM, SAMPLES_X1, SAMPLES_X4},
};

pub const VMM_SIZE_OPTIONS: [u32; 3] = [8, 16, 32];

// (workgroup size, per-thread sample layout) per cascade level.
const CASCADE_LAYOUT: [(u32, &str); GaussianRadianceBuffer::LEVELS as usize] = [
    (32, SAMPLES_X1),
    (32, SAMPLES_X1),
    (32, SAMPLES_X4),
    (64, SAMPLES_X4),
    (256, SAMPLES_X4),
    (1024, SAMPLES_X4),
];

pub struct LightingRenderer {
    variants: [Vec<ComputePipeline>; 3],
    active: Cell<usize>,
}

impl LightingRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let variants = VMM_SIZE_OPTIONS.map(|vmm_size| {
            (0..GaussianRadianceBuffer::LEVELS)
                .map(|cascade| {
                    gpu.compute(
                        "GaussianVolumeCascade",
                        &gpu.pipeline_layout(&[
                            &GaussianRadianceBuffer::layout_mipmap(gpu),
                            &PhysicalVolume::layout_read(gpu),
                            &Environment::layout(gpu),
                            &HdriBuffer::layout(gpu),
                        ]),
                        &gpu.shader(&vmm_template(&cascade_src(cascade), vmm_size)),
                    )
                })
                .collect()
        });

        let active = Cell::new(
            VMM_SIZE_OPTIONS
                .iter()
                .position(|&size| size == 32)
                .unwrap(),
        );

        Self { variants, active }
    }

    fn variant(&self) -> &[ComputePipeline] {
        &self.variants[self.active.get()]
    }

    pub fn set_vmm_size(&self, vmm_size: u32) {
        if let Some(index) = VMM_SIZE_OPTIONS.iter().position(|&size| size == vmm_size) {
            self.active.set(index);
        }
    }

    pub fn dispatch_for(
        &self,
        cmd: &mut CommandEncoder,
        asset: &Asset,
        controller: &Controller,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
    ) {
        self.set_vmm_size(controller.radiance().lobes());

        self.radiance(cmd, &asset.environment, &asset.hdri, radiance, volume);
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

        pass.set_pipeline(&self.variant()[cascade]);
        pass.set_bind_group(0, radiance.binding_mipmap(cascade), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        let probes = radiance.probes(cascade);
        pass.dispatch_workgroups(probes.x, probes.y, probes.z);
    }
}

fn cascade_src(cascade: u32) -> String {
    let (workgroup, samples) = CASCADE_LAYOUT[cascade as usize];

    (include_str!("cascade.wgsl").to_string() + samples + EM)
        .replace("#CASCADE", &cascade.to_string())
        .replace("#WORKGROUP", &workgroup.to_string())
}

fn vmm_template(src: &str, vmm_size: u32) -> String {
    src.replace("#VMM_SIZE", &vmm_size.to_string())
}
