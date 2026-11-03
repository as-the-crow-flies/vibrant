use itertools::Itertools;
use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline};

use crate::{
    asset::{
        environment::Environment, hdri::HdriBuffer, radiance::GaussianRadianceBuffer,
        volume::PhysicalVolume, Asset,
    },
    gpu::{Gpu, Lazy},
};

pub const VMM_SIZE_OPTIONS: [u32; 3] = [8, 16, 32];

// (workgroup size, per-thread sample layout) per cascade level.
const CASCADE_LAYOUT: [(u32, &str); GaussianRadianceBuffer::LEVELS as usize] = [
    (32, "renderer/lighting/cascade_x1.wgsl"),
    (32, "renderer/lighting/cascade_x1.wgsl"),
    (32, "renderer/lighting/cascade_x4.wgsl"),
    (64, "renderer/lighting/cascade_x4.wgsl"),
    (256, "renderer/lighting/cascade_x4.wgsl"),
    (1024, "renderer/lighting/cascade_x4.wgsl"),
];

// (workgroup size, shader, lobe count)
type CascadeKey = (u32, &'static str, u32);

pub struct LightingRenderer {
    // One pipeline per distinct `CASCADE_LAYOUT` entry and lobe count; the
    // level is read from the radiance buffer's `CASCADE_OPTS`.
    pipelines: Vec<(CascadeKey, Lazy<ComputePipeline>)>,
}

impl LightingRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = gpu.pipeline_layout(&[
            &GaussianRadianceBuffer::layout_mipmap(gpu),
            &PhysicalVolume::layout_read(gpu),
            &Environment::layout(gpu),
            &HdriBuffer::layout(gpu),
        ]);

        let pipelines = VMM_SIZE_OPTIONS
            .into_iter()
            .cartesian_product(CASCADE_LAYOUT.into_iter().unique())
            .map(|(vmm_size, (workgroup, shader))| {
                let layout = layout.clone();
                let pipeline = gpu.lazy(
                    format!("GaussianVolumeCascade ({shader}, {workgroup}, VMM {vmm_size})"),
                    move |gpu| {
                        gpu.compute(
                            "GaussianVolumeCascade",
                            &layout,
                            &gpu.shader_with(
                                shader,
                                &[("WORKGROUP", &workgroup), ("VMM_MAX", &vmm_size)],
                            ),
                        )
                    },
                );
                ((workgroup, shader, vmm_size), pipeline)
            })
            .collect();

        Self { pipelines }
    }

    pub fn dispatch_for(
        &self,
        cmd: &mut CommandEncoder,
        asset: &Asset,
        radiance: &GaussianRadianceBuffer,
        volume: &PhysicalVolume,
    ) {
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

        let (workgroup, shader) = CASCADE_LAYOUT[cascade];
        let key = (workgroup, shader, radiance.vmm_size());
        let (_, pipeline) = self
            .pipelines
            .iter()
            .find(|(layout, _)| *layout == key)
            .expect("cascade pipeline");

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, radiance.binding_mipmap(cascade), &[]);
        pass.set_bind_group(1, volume.binding_read(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.set_bind_group(3, hdri.binding(), &[]);

        let probes = radiance.probes(cascade);
        pass.dispatch_workgroups(probes.x, probes.y, probes.z);
    }
}
