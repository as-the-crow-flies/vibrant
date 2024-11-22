pub mod nifti;
pub mod tck;

pub use nifti::*;
pub use tck::*;

use std::{
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

#[derive(Default)]
pub struct File {
    pub tractogram: Option<Tck>,
    pub nifti: Option<Nifti>,
    pub save: Option<PathBuf>,
}

impl File {
    #[cfg(target_arch = "wasm32")]
    pub fn load() {
        wasm_bindgen_futures::spawn_local(async move {
            let file = rfd::AsyncFileDialog::new().pick_file().await;

            if let Some(file) = file {
                Self::publish_tractogram(Tck::from_bytes(&file.read().await));
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() {
        use std::{ffi::OsStr, fs};

        let files = rfd::FileDialog::new().pick_files();

        if let Some(files) = files {
            let tractograms: Vec<Tck> = files
                .iter()
                .filter(|file| file.extension() == Some(OsStr::new("tck")))
                .map(|file| Tck::from_bytes(&fs::read(file).unwrap()))
                .collect();

            if !tractograms.is_empty() {
                Self::publish_tractogram(Tck::join(tractograms));
            }

            if let Some(nifti) = files
                .iter()
                .filter(|file| file.extension() == Some(OsStr::new("gz")))
                .map(|file| Nifti::from_bytes(&fs::read(file).unwrap()))
                .next()
            {
                Self::publish_nifti(nifti);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save() {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("screenshot.png")
            .save_file()
        {
            Self::publish_save_path(path);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save() {
        todo!()
    }

    pub fn on_tck(callback: impl FnOnce(Tck)) {
        let mut data = QUEUE.lock().unwrap();

        if let Some(tractogram) = data.tractogram.take() {
            callback(tractogram);
        }
    }

    pub fn on_nifti(callback: impl FnOnce(Nifti)) {
        let mut data = QUEUE.lock().unwrap();

        if let Some(nifti) = data.nifti.take() {
            callback(nifti);
        }
    }

    pub fn about_to_save() -> bool {
        QUEUE.lock().unwrap().save.is_some()
    }

    pub fn on_save(callback: impl FnOnce(PathBuf)) {
        let mut data = QUEUE.lock().unwrap();

        if let Some(save) = data.save.take() {
            callback(save);
        }
    }

    fn publish_tractogram(tractogram: Tck) {
        QUEUE.lock().unwrap().tractogram = Some(tractogram);
    }

    fn publish_nifti(nifti: Nifti) {
        QUEUE.lock().unwrap().nifti = Some(nifti);
    }

    fn publish_save_path(path: PathBuf) {
        QUEUE.lock().unwrap().save = Some(path);
    }
}

static QUEUE: LazyLock<Mutex<File>> = LazyLock::new(|| Mutex::new(File::default()));
