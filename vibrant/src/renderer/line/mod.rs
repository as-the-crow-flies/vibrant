pub mod crop;
pub mod cull;
pub mod deposit;
pub mod occupancy;
pub mod populate;
pub mod render;
pub mod transform;

use occupancy::LineOccupancyPipeline;
use wgpu::CommandEncoder;

use crate::{
    asset::Asset,
    controller::Controller,
    gpu::Gpu,
    renderer::line::{
        crop::LineCropPipeline, cull::LineCullPipeline, deposit::LineDepositPipeline,
        populate::LinePopulatePipeline, render::LineRenderPipeline,
        transform::LineTransformPipeline,
    },
    surface::Frame,
};

use super::environment::Environment;

pub struct LineRenderer {
    transform: LineTransformPipeline,
    crop: LineCropPipeline,
    occupancy: LineOccupancyPipeline,
    cull: LineCullPipeline,
    deposit: LineDepositPipeline,
    populate: LinePopulatePipeline,
    render: LineRenderPipeline,
}

impl LineRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            transform: LineTransformPipeline::new(gpu),
            crop: LineCropPipeline::new(gpu),
            occupancy: LineOccupancyPipeline::new(gpu),
            cull: LineCullPipeline::new(gpu),
            deposit: LineDepositPipeline::new(gpu),
            populate: LinePopulatePipeline::new(gpu),
            render: LineRenderPipeline::new(gpu),
        }
    }

    /// Builds the tractography acceleration structure (transform/crop/occupancy/
    /// cull/populate). The occupancy density pyramid it produces is also what
    /// [`Self::deposit`] samples to bake line density into the shared
    /// [`PhysicalVolume`](crate::asset::volume::PhysicalVolume).
    pub fn transfer(
        &self,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        environment: &Environment,
        asset: &Asset,
    ) {
        if controller.tractography().visible() {
            if let Some(line) = &asset.line {
                self.transform.dispatch(cmd, line, environment);

                self.crop.dispatch(cmd, line, environment, &asset.crop);

                self.occupancy
                    .dispatch(cmd, environment, controller.settings(), line);

                self.cull.dispatch(cmd, line, environment);

                self.populate
                    .dispatch(cmd, environment, controller.settings(), line);
            }
        }
    }

    /// Writes neutral line density into the shared `PhysicalVolume`'s
    /// `line_extinction` side texture, so the radiance cascade(s) pick up the
    /// lines' occlusion. Never touches the marched volume textures.
    pub fn deposit(
        &self,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        environment: &Environment,
        asset: &Asset,
    ) {
        if !controller.tractography().visible() {
            return;
        }

        let (Some(line), Some(volume)) = (&asset.line, &asset.physical_volume) else {
            return;
        };

        self.deposit.dispatch(cmd, environment, volume, line);
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        controller: &Controller,
        environment: &Environment,
        asset: &Asset,
        frame: &Frame,
    ) {
        if !controller.tractography().visible() {
            return;
        }

        // X-ray+both -> the line-only cascade; otherwise the primary one.
        let radiance = asset.radiance_lines.as_ref().or(asset.radiance.as_ref());

        let (Some(line), Some(radiance)) = (&asset.line, radiance) else {
            return;
        };

        self.render.dispatch(
            cmd,
            frame,
            environment,
            controller.viewport(),
            controller.radiance().lobes(),
            controller.settings().render_mode,
            radiance,
            line,
        );
    }
}
