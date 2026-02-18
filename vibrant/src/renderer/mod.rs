pub mod environment;
pub mod line;
pub mod ui;
pub mod viewcube;
pub mod wgsl;

use std::sync::Arc;

use crate::{
    asset::{transform::TransformBuffer, volume::VolumeBuffer},
    file::bounds::Bounds,
    renderer::line::LineRenderer,
};
use environment::Environment;
use pollster::FutureExt;
use ui::UiRenderer;
use viewcube::ViewCubeRenderer;
use winit::window::Window;

use crate::{
    asset::{line::LineBuffer, Asset},
    file::FileStage,
};

use super::{controller::Controller, gpu::Gpu, surface::Surface};

pub struct Renderer {
    surface: Surface,
    egui: egui_winit::State,
    line: LineRenderer,
    ui: UiRenderer,
    viewcube: ViewCubeRenderer,
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
            line: LineRenderer::new(gpu),
            ui: UiRenderer::new(gpu),
            viewcube: ViewCubeRenderer::new(gpu, viewcube::VIEWCUBE_SIZE),

            environment: Environment::new(gpu),
            asset: Asset::default(),
        }
    }

    pub fn egui(&mut self) -> &mut egui_winit::State {
        &mut self.egui
    }

    pub fn has_assets(&self) -> bool {
        self.asset.line.is_some()
    }

    /// Read back the current frame as raw RGBA bytes. Returns (data, width, height).
    pub async fn read_frame(&self, gpu: &Gpu) -> (Vec<u8>, u32, u32) {
        gpu.read_frame(self.surface.buffer().color().texture())
            .await
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
            self.asset
                .volumes
                .extend(volumes.iter().map(|volume| VolumeBuffer::new(gpu, volume)));

            if let Some(volume) = volumes.last() {
                self.asset.transform = Some(TransformBuffer::new(gpu, volume.transform()));
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

        let mut cmd = gpu.cmd();

        if let (Some(line), Some(transform)) = (&self.asset.line, &self.asset.transform) {
            self.line.render(
                &mut cmd,
                &self.environment,
                surface.buffer(),
                line,
                transform,
                controller.settings(),
                needs_transform,
                needs_update,
            );
        }

        // Render view cube on top of the post-processed buffer
        {
            let camera_rotation = controller.camera().rotation();
            let sw = controller.settings().width;
            let sh = controller.settings().height;
            let hovered_id = controller.viewcube_hovered_id();

            self.viewcube.render(
                gpu,
                &mut cmd,
                surface.buffer().post().view(),
                camera_rotation,
                sw,
                sh,
                hovered_id,
            );

            self.viewcube.render_pick(gpu, &mut cmd, camera_rotation);
        }

        // Submit pick pass and handle viewcube click/hover readback
        // We need to submit the pick pass commands before reading back
        gpu.submit(cmd);
        controller.handle_viewcube_pick(&self.viewcube, gpu);
        let mut cmd = gpu.cmd();

        if !FileStage::about_to_save() {
            self.ui.render(
                gpu,
                &mut cmd,
                surface.buffer(),
                self.egui.egui_ctx(),
                output,
            );
        }

        surface.present(gpu, cmd);

        FileStage::on_save(|path| {
            gpu.save(path, surface.buffer().color().texture())
                .block_on()
        });
    }
}
