pub mod deposit;
pub mod occupancy;
pub mod populate;
pub mod prepare;
pub mod render;

use occupancy::LineOccupancyPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::Asset,
    controller::Controller,
    gpu::Gpu,
    renderer::line::{
        deposit::LineDepositPipeline, populate::LinePopulatePipeline, prepare::LinePreparePipeline,
        render::LineRenderPipeline,
    },
    surface::Frame,
};

pub struct LineRenderer {
    prepare: LinePreparePipeline,
    occupancy: LineOccupancyPipeline,
    deposit: LineDepositPipeline,
    populate: LinePopulatePipeline,
    render: LineRenderPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            prepare: LinePreparePipeline::new(gpu),
            occupancy: LineOccupancyPipeline::new(gpu),
            deposit: LineDepositPipeline::new(gpu),
            populate: LinePopulatePipeline::new(gpu),
            render: LineRenderPipeline::new(gpu),
        }
    }

    /// Builds the tractography acceleration structure (prepare/occupancy/populate). The occupancy density pyramid it produces is also what
    /// [`Self::deposit`] samples to bake line density into the shared
    /// [`PhysicalVolume`](crate::asset::volume::PhysicalVolume).
    pub fn transfer(&self, cmd: &mut CommandEncoder, controller: &Controller, asset: &Asset) {
        if controller.tractography().visible() {
            if let Some(line) = &asset.line {
                self.prepare.dispatch(cmd, asset, line);

                self.occupancy.dispatch(cmd, asset, controller, line);

                self.populate.dispatch(cmd, asset, controller, line);
            }
        }
    }

    /// Writes neutral line density into the shared `PhysicalVolume`'s
    /// `line_extinction` side texture, so the radiance cascade(s) pick up the
    /// lines' occlusion. Never touches the marched volume textures.
    pub fn deposit(&self, cmd: &mut CommandEncoder, controller: &Controller, asset: &Asset) {
        if !controller.tractography().visible() {
            return;
        }

        let (Some(line), Some(volume)) = (&asset.line, &asset.physical_volume) else {
            return;
        };

        self.deposit
            .dispatch(cmd, volume, line, &asset.tractography);
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        asset: &Asset,
        frame: &Frame,
    ) {
        if !controller.tractography().visible() {
            return;
        }

        self.render.dispatch(cmd, asset, controller, frame);
    }
}
