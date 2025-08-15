use wgpu::{CommandEncoder, FilterMode};

use crate::{
    asset::texture::{MipTexture2D, R32Float},
    gpu::Gpu,
};

pub struct OpacityBuffer {
    opacity: MipTexture2D<R32Float>,
}

impl OpacityBuffer {
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        Self {
            opacity: MipTexture2D::new(
                gpu,
                width.div_ceil(2),
                height.div_ceil(2),
                1,
                FilterMode::Linear,
            ),
        }
    }

    pub fn clear(&self, cmd: &mut CommandEncoder) {
        self.opacity.clear(cmd);
    }

    pub fn opacity(&self) -> &MipTexture2D<R32Float> {
        &self.opacity
    }
}
