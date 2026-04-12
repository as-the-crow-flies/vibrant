pub mod anatomy;
pub mod environment;
pub mod line;
pub mod ui;
pub mod wgsl;

use std::sync::Arc;

use crate::{
    asset::{
        hdri::HdriBuffer, radiance::RadianceVolume, transform::TransformBuffer,
        volume::PhysicalVolume, volume_fraction::VolumeFractionBuffer,
    },
    file::bounds::Bounds,
    renderer::{anatomy::AnatomyRenderer, line::LineRenderer},
};
use environment::Environment;
use pollster::FutureExt;
use ui::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{line::LineBuffer, Asset},
    file::FileStage,
};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    surface: Surface,
    egui: egui_winit::State,

    anatomy: AnatomyRenderer,
    line: LineRenderer,
    ui: UiRenderer,

    environment: Environment,
    asset: Asset,
}

impl Renderer {
    pub fn new(gpu: &Gpu, window: Arc<Window>) -> Self {
        Self {
            egui: egui_winit::State::new(
                egui::Context::default(),
                egui::viewport::ViewportId::ROOT,
                &window,
                Some(window.scale_factor() as f32),
                None,
                None,
            ),
            surface: Surface::new(gpu, window),

            anatomy: AnatomyRenderer::new(gpu),
            line: LineRenderer::new(gpu),
            ui: UiRenderer::new(gpu),

            environment: Environment::new(gpu),
            asset: Asset::default(),
        }
    }

    pub fn egui(&mut self) -> &mut egui_winit::State {
        &mut self.egui
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        window: &Arc<Window>,
        controller: &mut Controller,
        dt: f32,
    ) {
        let mut needs_transform = false;
        let needs_update = true;

        FileStage::on_lines(|lines| {
            self.asset.line = Some(LineBuffer::new(gpu, &lines));

            let bounds: Vec<Bounds> = lines.iter().map(|line| line.bounds()).copied().collect();
            let bounds = Bounds::from_bounds(&bounds);

            self.asset.transform = Some(TransformBuffer::new(gpu, bounds.transform().inverse()));

            needs_transform = true;
        });

        FileStage::on_volumes(|volumes| {
            self.asset.volumes.extend(
                volumes
                    .iter()
                    .map(|volume| VolumeFractionBuffer::new(gpu, volume)),
            );

            if let Some(volume) = volumes.last() {
                self.asset.transform = Some(TransformBuffer::new(gpu, volume.transform()));

                if self.asset.physical_volume.is_none() {
                    self.asset.physical_volume =
                        Some(PhysicalVolume::new(gpu, volume.size(), volume.transform()));

                    self.asset.radiance = Some(RadianceVolume::new(gpu, volume.size()))
                }

                if self.asset.hdri.is_none() {
                    self.asset.hdri = Some(HdriBuffer::white(gpu))
                }
            }
        });

        FileStage::on_hdris(|hdris| {
            for hdri in hdris {
                self.asset.hdri = Some(HdriBuffer::from_file(gpu, &hdri));
            }
        });

        let surface = self.surface.maybe_resize(gpu, &controller.settings());

        let input = self.egui.take_egui_input(window);
        let output = self
            .egui
            .egui_ctx()
            .run(input, |ctx| controller.ui(ctx, &mut self.asset, dt));
        self.egui
            .handle_platform_output(&window, output.platform_output.clone());

        self.environment.update(gpu, &controller);

        if let Some(line) = &self.asset.line {
            line.update_settings(gpu);
        }
        if let Some(hdri) = &self.asset.hdri {
            hdri.update_settings(gpu);
        }
        for volume in &self.asset.volumes {
            volume.update_settings(gpu);
        }

        let mut cmd = gpu.cmd();

        self.anatomy.render(
            &mut cmd,
            controller,
            &self.environment,
            surface.frame(),
            &self.asset,
        );

        if let (Some(line), Some(transform)) = (&self.asset.line, &self.asset.transform) {
            self.line.render(
                &mut cmd,
                &self.environment,
                surface.frame(),
                line,
                transform,
                controller.settings(),
                needs_transform,
                needs_update,
            );
        }

        if !FileStage::about_to_save() {
            self.ui
                .render(gpu, &mut cmd, surface.frame(), self.egui.egui_ctx(), output);
        }

        surface.present(gpu, cmd);

        FileStage::on_save(|path| gpu.save(path, surface.frame().color().texture()).block_on());
    }
}
