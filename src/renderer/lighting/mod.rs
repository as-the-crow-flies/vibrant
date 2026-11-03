use std::{cell::Cell, rc::Rc};

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        environment::Environment, hdri::HdriBuffer, radiance::GaussianRadianceBuffer,
        volume::PhysicalVolume, Asset,
    },
    controller::Controller,
    gpu::{Gpu, Lazy},
};

pub const VMM_SIZE_OPTIONS: [u32; 3] = [8, 16, 32];

/// One pipeline (set) per `VMM_SIZE_OPTIONS` entry, compiled on first use so
/// startup only pays for the active lobe count.
pub struct VmmVariants<P> {
    variants: [Lazy<P>; 3],
    active: Cell<usize>,
}

impl<P: 'static> VmmVariants<P> {
    pub fn new(gpu: &Gpu, label: &'static str, build: impl Fn(&Gpu, u32) -> P + 'static) -> Self {
        let build = Rc::new(build);

        Self {
            variants: VMM_SIZE_OPTIONS.map(|vmm_size| {
                let build = Rc::clone(&build);
                gpu.lazy(format!("{label} (VMM {vmm_size})"), move |gpu| {
                    build(gpu, vmm_size)
                })
            }),
            active: Cell::new(VMM_SIZE_OPTIONS.len() - 1),
        }
    }

    pub fn set_vmm_size(&self, vmm_size: u32) {
        if let Some(index) = VMM_SIZE_OPTIONS.iter().position(|&size| size == vmm_size) {
            self.active.set(index);
        }
    }

    pub fn active(&self) -> &P {
        &self.variants[self.active.get()]
    }
}

// (workgroup size, per-thread sample layout) per cascade level.
const CASCADE_LAYOUT: [(u32, &str); GaussianRadianceBuffer::LEVELS as usize] = [
    (32, "renderer/lighting/cascade_x1.wgsl"),
    (32, "renderer/lighting/cascade_x1.wgsl"),
    (32, "renderer/lighting/cascade_x4.wgsl"),
    (64, "renderer/lighting/cascade_x4.wgsl"),
    (256, "renderer/lighting/cascade_x4.wgsl"),
    (1024, "renderer/lighting/cascade_x4.wgsl"),
];

pub struct LightingRenderer {
    variants: VmmVariants<Vec<ComputePipeline>>,
}

impl LightingRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = gpu.pipeline_layout(&[
            &GaussianRadianceBuffer::layout_mipmap(gpu),
            &PhysicalVolume::layout_read(gpu),
            &Environment::layout(gpu),
            &HdriBuffer::layout(gpu),
        ]);

        let variants = VmmVariants::new(gpu, "GaussianVolumeCascade", move |gpu, vmm_size| {
            (0..GaussianRadianceBuffer::LEVELS)
                .map(|cascade| {
                    let (workgroup, shader) = CASCADE_LAYOUT[cascade as usize];

                    gpu.compute(
                        "GaussianVolumeCascade",
                        &layout,
                        &gpu.shader_with(
                            shader,
                            &[
                                ("CASCADE", &cascade),
                                ("WORKGROUP", &workgroup),
                                ("VMM_SIZE", &vmm_size),
                            ],
                        ),
                    )
                })
                .collect()
        });

        Self { variants }
    }

    pub fn set_vmm_size(&self, vmm_size: u32) {
        self.variants.set_vmm_size(vmm_size);
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

        pass.set_pipeline(&self.variants.active()[cascade]);
        pass.set_bind_group(0, radiance.binding_mipmap(cascade), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        let probes = radiance.probes(cascade);
        pass.dispatch_workgroups(probes.x, probes.y, probes.z);
    }
}
