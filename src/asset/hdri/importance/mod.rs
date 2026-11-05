mod grid;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::{
    gpu::{Gpu, Lazy},
    renderer::lighting::VMM_SIZE_OPTIONS,
};

pub use grid::Grid;

/// Bakes the environment VMM for one lobe count: K seed lobes cut from the
/// HDRI's equal-area grid, refined by an EM fit over its cells. Lobe count K
/// lives at `[K, 2K)` of each buffer.
pub struct Importance {
    fit: Lazy<ComputePipeline>,
}

/// World-space lobes and their radiance at unit strength.
pub struct Lobes {
    pub vmm: Buffer,
    pub phi: Buffer,
}

/// How the grid is cut into cells.
#[derive(Clone, Copy, PartialEq)]
pub struct Cut {
    /// Cut by log(1 + Y) instead of Y.
    pub log_samples: bool,
    /// Share of the cut spread evenly over the sphere.
    pub uniform_share: f32,
    /// Weight the EM fits by log(1 + Y) instead of Y.
    pub log_em: bool,
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
    const LOBES: u32 = VMM_SIZE_OPTIONS[VMM_SIZE_OPTIONS.len() - 1];
    /// The largest cascade sample count, and the fit's.
    const SAMPLES: u32 = 4096;

    pub fn new(gpu: &Gpu) -> Self {
        Self {
            fit: gpu.lazy("Hdri::Importance::Fit", |gpu| {
                gpu.device()
                    .create_compute_pipeline(&ComputePipelineDescriptor {
                        label: Some("Hdri::Importance::Fit"),
                        layout: None,
                        module: &gpu.shader("asset/hdri/importance/fit.wgsl"),
                        entry_point: None,
                        compilation_options: Default::default(),
                        cache: None,
                    })
            }),
        }
    }

    /// Cuts `grid` into `cells` (read by the fit and the cascade), then fits
    /// `vmm_size` lobes to them.
    pub fn bake(
        &self,
        gpu: &Gpu,
        grid: &Grid,
        cells: &Buffer,
        lobes: &Lobes,
        cut: Cut,
        vmm_size: u32,
    ) {
        let cell_data = grid.cells(
            Self::SAMPLES,
            cut.log_samples,
            cut.uniform_share,
            cut.log_em,
        );
        gpu.queue()
            .write_buffer(cells, 0, bytemuck::cast_slice(&cell_data));

        let seed = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("Hdri::Seed"),
            contents: bytemuck::cast_slice(&grid.seeds(Self::LOBES, cut.log_em)),
            usage: BufferUsages::STORAGE,
        });

        let vmm_size = gpu.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("Hdri::Importance::Fit"),
            contents: bytemuck::bytes_of(&vmm_size),
            usage: BufferUsages::UNIFORM,
        });

        let binding = gpu.binding(
            "Hdri::Importance::Fit",
            &self.fit.get_bind_group_layout(0),
            vec![
                cells.as_entire_binding(),
                seed.as_entire_binding(),
                lobes.vmm.as_entire_binding(),
                lobes.phi.as_entire_binding(),
                vmm_size.as_entire_binding(),
            ],
        );

        let mut cmd = gpu.cmd();
        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&self.fit);
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups(1, 1, 1);

        drop(pass);
        gpu.submit(cmd);
    }
}

/// N cells live at `[N, 2N)` for every power-of-two N up to the largest cascade.
pub fn cell_buffer(gpu: &Gpu, label: &str) -> Buffer {
    gpu.device().create_buffer(&BufferDescriptor {
        label: Some(label),
        size: 2 * Importance::SAMPLES as u64 * size_of::<grid::Cell>() as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn lobe_buffer(gpu: &Gpu, label: &str) -> Buffer {
    gpu.device().create_buffer(&BufferDescriptor {
        label: Some(label),
        size: 2 * Importance::LOBES as u64 * 16,
        usage: BufferUsages::STORAGE,
        mapped_at_creation: false,
    })
}
