use wgpu::BindGroup;

use crate::gpu::Gpu;

use super::scalar::ScalarTexture;

pub struct Occlusion {
    ping: ScalarTexture,
    pong: ScalarTexture,
}

impl Occlusion {
    pub fn new(gpu: &Gpu, exponent: u32) -> Self {
        Self {
            ping: ScalarTexture::new(gpu, exponent),
            pong: ScalarTexture::new(gpu, exponent),
        }
    }

    pub fn binding(&self) -> &BindGroup {
        self.ping.binding()
    }

    pub fn bindings_mipmap(&self) -> &[BindGroup] {
        &self.ping.bindings_mipmap()
    }

    pub fn ping(&self) -> &ScalarTexture {
        &self.ping
    }

    pub fn pong(&self) -> &ScalarTexture {
        &self.pong
    }
}
