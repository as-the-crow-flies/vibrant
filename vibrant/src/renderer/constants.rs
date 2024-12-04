use crate::surface::kbuffer::KBuffer;

#[derive(Debug, Clone, Copy)]
pub struct Constants {
    pub surface_x: u32,
    pub surface_y: u32,
    pub volume_xyz: u32,
    pub workgroup_x: u32,
    pub workgroup_xy: u32,
    pub workgroup_xyz: u32,
    pub layers: u32,
}

impl Constants {
    pub fn new(surface: (u32, u32), volume: u32) -> Self {
        Self {
            surface_x: surface.0,
            surface_y: surface.1,
            volume_xyz: volume,
            workgroup_x: 256,
            workgroup_xy: 16,
            workgroup_xyz: 4,
            layers: KBuffer::LAYERS,
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
            + &format!("const LAYERS: u32 = {:};\n", self.layers)
            + &format!("const VOLUME_XYZ: u32 = {:};\n", self.volume_xyz)
            + &format!("const WORKGROUP_X: u32 = {:};\n", self.workgroup_x)
            + &format!("const WORKGROUP_XY: u32 = {:};\n", self.workgroup_xy)
            + &format!("const WORKGROUP_XYZ: u32 = {:};\n\n", self.workgroup_xyz)
    }
}
