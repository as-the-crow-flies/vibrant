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
        hdri::HdriBuffer, radiance::RadianceVolume, volume_fraction::VolumeFractionBuffer,
        volume_mask::VolumeMaskBuffer,
    },
    file::FileStage,
    gpu::Gpu,
};

#[derive(Default)]
pub struct Asset {
    pub line: Option<LineBuffer>,
    pub volumes: Vec<VolumeFractionBuffer>,
    pub mask: Option<VolumeMaskBuffer>,
    pub physical_volume: Option<PhysicalVolume>,
    pub radiance: Option<RadianceVolume>,
    pub hdri: Option<HdriBuffer>,
}

impl Asset {
    pub fn update(&mut self, gpu: &Gpu) {
        FileStage::on_lines(|lines| {
            let line = LineBuffer::new(gpu, &lines);

            if let Some(volume) = self.volumes.last() {
                line.set_transform(gpu, &volume.transform());
            }

            self.line = Some(line);
        });

        FileStage::on_volumes(|volumes| {
            for volume in &volumes {
                if volume.name().contains("mask") {
                    self.mask = Some(VolumeMaskBuffer::new(gpu, volume));
                } else {
                    self.volumes.push(VolumeFractionBuffer::new(gpu, volume));
                }
            }

            if let Some(volume) = volumes.last() {
                if let Some(line) = &self.line {
                    line.set_transform(gpu, &volume.transform());
                }

                if self.physical_volume.is_none() {
                    self.physical_volume =
                        Some(PhysicalVolume::new(gpu, volume.size(), volume.transform()));

                    self.radiance = Some(RadianceVolume::new(gpu, volume.size()))
                }

                if self.mask.is_none() {
                    self.mask = Some(VolumeMaskBuffer::white(gpu));
                }

                if self.hdri.is_none() {
                    self.hdri = Some(HdriBuffer::white(gpu))
                }
            }
        });

        FileStage::on_hdris(|hdris| {
            for hdri in hdris {
                self.hdri = Some(HdriBuffer::from_file(gpu, &hdri));
            }
        });
    }
}
