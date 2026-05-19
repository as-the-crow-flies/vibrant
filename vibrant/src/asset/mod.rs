pub mod colormap;
pub mod crop;
pub mod hdri;
pub mod line;
pub mod radiance;
pub mod texture;
pub mod volume;
pub mod volume_fraction;
pub mod volume_mask;

use line::LineBuffer;
use volume::PhysicalVolume;

use crate::{
    asset::{
        colormap::Colormap, crop::CropBuffer, hdri::HdriBuffer, radiance::RadianceVolume,
        volume_fraction::VolumeFractionBuffer, volume_mask::VolumeMaskBuffer,
    },
    file::FileStage,
    gpu::Gpu,
};

pub struct Asset {
    pub colormap: Colormap,
    pub hdri: HdriBuffer,
    pub crop: CropBuffer,

    pub line: Option<LineBuffer>,
    pub volumes: Vec<VolumeFractionBuffer>,
    pub masks: Vec<VolumeMaskBuffer>,
    pub physical_volume: Option<PhysicalVolume>,
    pub radiance: Option<RadianceVolume>,

    pub changed: bool,
}

impl Asset {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            colormap: Colormap::new(gpu),
            crop: CropBuffer::new(gpu),
            hdri: HdriBuffer::new(gpu),
            masks: vec![VolumeMaskBuffer::none(gpu)],

            line: None,
            volumes: Vec::new(),
            physical_volume: None,
            radiance: None,
            changed: false,
        }
    }

    pub fn update(&mut self, gpu: &Gpu) {
        self.changed = false;

        FileStage::on_lines(|lines| {
            let line = LineBuffer::new(gpu, &lines, &self.colormap);

            if let Some(volume) = self
                .volumes
                .iter()
                .max_by_key(|volume| volume.size().element_product())
            {
                line.set_transform(gpu, &volume.transform());
            }

            self.line = Some(line);

            self.changed = true;
        });

        FileStage::on_track_scalars(|track_scalars| {
            if let Some(line) = &mut self.line {
                for scalar in track_scalars {
                    line.set_scalar(gpu, scalar);
                }
            }
        });

        FileStage::on_volumes(|volumes| {
            for volume in &volumes {
                if volume.name().contains("mask") {
                    self.masks.push(VolumeMaskBuffer::new(gpu, volume));
                } else {
                    self.volumes
                        .push(VolumeFractionBuffer::new(gpu, volume, &self.colormap));
                }
            }

            if let Some(volume) = self
                .volumes
                .iter()
                .max_by_key(|volume| volume.size().element_product())
            {
                if let Some(line) = &self.line {
                    line.set_transform(gpu, &volume.transform());
                }

                self.physical_volume =
                    Some(PhysicalVolume::new(gpu, volume.size(), volume.transform()));

                self.radiance = Some(RadianceVolume::new(gpu, volume.size()));

                self.changed = true;
            }
        });

        FileStage::on_hdris(|hdris| {
            for hdri in hdris {
                self.hdri.import(gpu, hdri);
            }

            self.changed = true;
        });

        self.hdri.update_settings(gpu);
        self.crop.update_settings(gpu);

        if let Some(line) = &self.line {
            line.update_settings(gpu);
        }
        for volume in &self.volumes {
            volume.update_settings(gpu);
        }
        for mask in &self.masks {
            mask.update_settings(gpu);
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
