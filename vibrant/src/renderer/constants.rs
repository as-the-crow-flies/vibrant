use crate::gpu::Gpu;

pub struct Constants {
    pub surface_x: u32,
    pub surface_y: u32,
    pub volume_xyz: u32,
    pub workgroup_x: u32,
    pub workgroup_xy: u32,
    pub workgroup_xyz: u32,
}

impl Constants {
    pub fn new(gpu: &Gpu, surface: (u32, u32), volume: u32) -> Self {
        let limits = gpu.device().limits();

        Self {
            surface_x: surface.0,
            surface_y: surface.1,
            volume_xyz: volume,
            workgroup_x: limits
                .max_compute_invocations_per_workgroup
                .min(limits.max_compute_workgroup_size_x),
            workgroup_xy: sqrt(limits.max_compute_invocations_per_workgroup)
                .min(limits.max_compute_workgroup_size_x)
                .min(limits.max_compute_workgroup_size_y),
            workgroup_xyz: cbrt(limits.max_compute_invocations_per_workgroup)
                .min(limits.max_compute_workgroup_size_x)
                .min(limits.max_compute_workgroup_size_y)
                .min(limits.max_compute_workgroup_size_z),
        }
    }

    pub fn num_workgroups_xy(&self) -> (u32, u32) {
        (
            self.surface_x.div_ceil(self.workgroup_xy),
            self.surface_y.div_ceil(self.workgroup_xy),
        )
    }

    pub fn num_workgroups_xyz(&self) -> (u32, u32, u32) {
        (
            self.surface_x.div_ceil(self.workgroup_xyz),
            self.surface_y.div_ceil(self.workgroup_xyz),
            self.surface_y.div_ceil(self.workgroup_xyz),
        )
    }

    pub fn wgsl(&self) -> String {
        "// Constants\n".to_string()
            + &format!("const SURFACE_X: u32 = {:};\n", self.surface_x)
            + &format!("const SURFACE_Y: u32 = {:};\n", self.surface_y)
            + &format!("const VOLUME_XYZ: u32 = {:};\n", self.volume_xyz)
            + &format!("const WORKGROUP_X: u32 = {:};\n", self.workgroup_x)
            + &format!("const WORKGROUP_XY: u32 = {:};\n", self.workgroup_xy)
            + &format!("const WORKGROUP_XYZ: u32 = {:};\n\n", self.workgroup_xyz)
    }
}

fn sqrt(x: u32) -> u32 {
    (x as f32).sqrt() as u32
}

fn cbrt(x: u32) -> u32 {
    (x as f32).powf(1.0 / 3.0) as u32
}
