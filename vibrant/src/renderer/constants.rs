use crate::gpu::Gpu;

#[derive(Debug, Clone, Copy)]
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
            workgroup_x: limits.max_compute_workgroup_size_x,
            workgroup_xy: 16,
            workgroup_xyz: 4,
        }
    }

    pub fn num_workgroups_surface(&self) -> (u32, u32) {
        (
            self.surface_x.div_ceil(self.workgroup_xy),
            self.surface_y.div_ceil(self.workgroup_xy),
        )
    }

    pub fn num_workgroups_volume(&self) -> u32 {
        self.volume_xyz.div_ceil(self.workgroup_xyz)
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
