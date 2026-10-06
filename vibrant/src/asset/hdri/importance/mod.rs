use wgpu::*;

use crate::{
    gpu::Gpu,
    renderer::{
        lighting::VMM_SIZE_OPTIONS,
        wgsl::em::{EM, SAMPLES_X4},
    },
};

/// Bakes the environment VMM for every lobe count: K seed lobes, either cut
/// from an equal-area luminance hierarchy or at uniform Hammersley directions,
/// refined by an EM fit. Lobe count K lives at `[K, 2K)` of each buffer.
pub struct Importance {
    resample: ComputePipeline,
    mipmap: ComputePipeline,
    cut: ComputePipeline,
    uniform: ComputePipeline,
    fit: Vec<ComputePipeline>,
}

/// World-space lobes and their radiance at unit strength.
pub struct Lobes {
    pub vmm: Buffer,
    pub phi: Buffer,
}

impl Lobes {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            vmm: lobe_buffer(gpu, "Hdri::Vmm"),
            phi: lobe_buffer(gpu, "Hdri::Phi"),
        }
    }
}

impl Importance {
    const SIZE: u32 = 256;
    const LOBES: u32 = VMM_SIZE_OPTIONS[VMM_SIZE_OPTIONS.len() - 1];

    pub fn new(gpu: &Gpu) -> Self {
        let cut = include_str!("cut.wgsl").replace("#LOBES", &Self::LOBES.to_string());
        let fit = include_str!("fit.wgsl").to_string() + SAMPLES_X4 + EM;

        Self {
            resample: pipeline(
                gpu,
                "Hdri::Importance::Resample",
                include_str!("resample.wgsl"),
            ),
            mipmap: pipeline(gpu, "Hdri::Importance::Mipmap", include_str!("mipmap.wgsl")),
            cut: pipeline(gpu, "Hdri::Importance::Cut", &cut),
            uniform: pipeline(
                gpu,
                "Hdri::Importance::Uniform",
                include_str!("uniform.wgsl"),
            ),
            fit: VMM_SIZE_OPTIONS
                .map(|vmm_size| {
                    let src = fit.replace("#VMM_SIZE", &vmm_size.to_string());
                    pipeline(gpu, "Hdri::Importance::Fit", &src)
                })
                .to_vec(),
        }
    }

    pub fn bake(&self, gpu: &Gpu, hdri: &Texture, sampler: &Sampler, lobes: &Lobes, cut: bool) {
        let seed = lobe_buffer(gpu, "Hdri::Seed");
        let hdri = hdri.create_view(&TextureViewDescriptor::default());

        let mut cmd = gpu.cmd();
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        if cut {
            self.cut(gpu, &mut pass, &hdri, sampler, &seed);
        } else {
            self.uniform(gpu, &mut pass, &seed);
        }

        for fit in &self.fit {
            let binding = gpu.binding(
                "Hdri::Importance::Fit",
                &fit.get_bind_group_layout(0),
                vec![
                    BindingResource::TextureView(&hdri),
                    BindingResource::Sampler(sampler),
                    seed.as_entire_binding(),
                    lobes.vmm.as_entire_binding(),
                    lobes.phi.as_entire_binding(),
                ],
            );

            pass.set_pipeline(fit);
            pass.set_bind_group(0, &binding, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        drop(pass);
        gpu.submit(cmd);
    }

    fn cut(
        &self,
        gpu: &Gpu,
        pass: &mut ComputePass,
        hdri: &TextureView,
        sampler: &Sampler,
        seed: &Buffer,
    ) {
        let mip_level_count = Self::SIZE.ilog2() + 1;

        let texture = gpu.device().create_texture(&TextureDescriptor {
            label: Some("Hdri::Importance"),
            size: Extent3d {
                width: Self::SIZE,
                height: Self::SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let level = |level: u32| {
            texture.create_view(&TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };

        let binding = gpu.binding(
            "Hdri::Importance::Resample",
            &self.resample.get_bind_group_layout(0),
            vec![
                BindingResource::TextureView(hdri),
                BindingResource::Sampler(sampler),
                BindingResource::TextureView(&level(0)),
            ],
        );

        pass.set_pipeline(&self.resample);
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups(Self::SIZE.div_ceil(8), Self::SIZE.div_ceil(8), 1);

        pass.set_pipeline(&self.mipmap);

        for level_index in 1..mip_level_count {
            let size = Self::SIZE >> level_index;

            let binding = gpu.binding(
                "Hdri::Importance::Mipmap",
                &self.mipmap.get_bind_group_layout(0),
                vec![
                    BindingResource::TextureView(&level(level_index - 1)),
                    BindingResource::TextureView(&level(level_index)),
                ],
            );

            pass.set_bind_group(0, &binding, &[]);
            pass.dispatch_workgroups(size.div_ceil(8), size.div_ceil(8), 1);
        }

        let binding = gpu.binding(
            "Hdri::Importance::Cut",
            &self.cut.get_bind_group_layout(0),
            vec![
                BindingResource::TextureView(
                    &texture.create_view(&TextureViewDescriptor::default()),
                ),
                seed.as_entire_binding(),
            ],
        );

        pass.set_pipeline(&self.cut);
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    fn uniform(&self, gpu: &Gpu, pass: &mut ComputePass, seed: &Buffer) {
        let binding = gpu.binding(
            "Hdri::Importance::Uniform",
            &self.uniform.get_bind_group_layout(0),
            vec![seed.as_entire_binding()],
        );

        pass.set_pipeline(&self.uniform);
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups((2 * Self::LOBES).div_ceil(64), 1, 1);
    }
}

fn lobe_buffer(gpu: &Gpu, label: &str) -> Buffer {
    gpu.device().create_buffer(&BufferDescriptor {
        label: Some(label),
        size: 2 * Importance::LOBES as u64 * 16,
        usage: BufferUsages::STORAGE,
        mapped_at_creation: false,
    })
}

fn pipeline(gpu: &Gpu, label: &str, src: &str) -> ComputePipeline {
    gpu.device()
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(label),
            layout: None,
            module: &gpu.shader(src),
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        })
}
