pub mod bounds;
pub mod hdri;
pub mod line;
pub mod volume;

pub use line::*;
pub use volume::*;

use std::{
    fs::{self},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        LazyLock, Mutex,
    },
};

use log::warn;

use crate::file::hdri::HdriFile;

pub struct File {
    name: String,
    data: Vec<u8>,
}

impl File {
    pub fn new(name: &str, data: Vec<u8>) -> Self {
        Self {
            name: name.to_string(),
            data,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}

impl From<&str> for File {
    fn from(value: &str) -> Self {
        File {
            name: value.to_string(),
            data: fs::read(value).expect("Could not read file"),
        }
    }
}

impl From<&PathBuf> for File {
    fn from(path: &PathBuf) -> Self {
        Self {
            name: path.file_name().unwrap().to_str().unwrap().to_owned(),
            data: fs::read(path)
                .unwrap_or_else(|_| panic!("should be able to read path: `{:?}`", path.to_str())),
        }
    }
}

#[derive(Default)]
pub struct FileStage {
    pub lines: Vec<LineFile>,
    pub track_scalars: Vec<TrackScalarFile>,
    pub volumes: Vec<VolumeFile>,
    pub hdris: Vec<HdriFile>,
    pub save: Option<PathBuf>,
}

impl FileStage {
    /// Register a callback that nudges the event loop into rendering a frame,
    /// so the loading spinner animates while files parse off the main thread
    /// (native) or off a resolved promise (web).
    pub fn set_waker(waker: impl Fn() + Send + 'static) {
        if let Ok(mut slot) = WAKER.lock() {
            *slot = Some(Box::new(waker));
        }
    }

    /// Whether a file load is in progress: dialog committed, bytes still being
    /// parsed, or the result not yet drained into GPU resources by `Asset`.
    pub fn loading() -> bool {
        LOADING.load(Ordering::Relaxed)
    }

    fn set_loading(loading: bool) {
        LOADING.store(loading, Ordering::Relaxed);
        if let Ok(slot) = WAKER.lock() {
            if let Some(waker) = slot.as_ref() {
                waker();
            }
        }
    }

    /// True while the queue still holds files `Asset` hasn't consumed.
    pub fn has_pending() -> bool {
        QUEUE
            .lock()
            .map(|stage| {
                !stage.lines.is_empty()
                    || !stage.track_scalars.is_empty()
                    || !stage.volumes.is_empty()
                    || !stage.hdris.is_empty()
            })
            .unwrap_or(false)
    }

    /// Called by `Asset` once it has drained the queue, so the spinner clears
    /// only after the loaded data is actually built.
    pub fn finish_loading() {
        Self::set_loading(false);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() {
        wasm_bindgen_futures::spawn_local(async move {
            let Some(handles) = rfd::AsyncFileDialog::new().pick_files().await else {
                return;
            };

            Self::set_loading(true);

            let mut files: Vec<File> = Vec::new();
            for handle in handles {
                files.push(File {
                    name: handle.file_name(),
                    data: handle.read().await,
                });
            }

            Self::load_files(files);
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() {
        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            Self::spawn_load(move || paths.iter().map(|path| path.into()).collect());
        }
    }

    pub fn load_path(path: &Path) {
        let path = path.to_path_buf();
        Self::spawn_load(move || vec![File::from(&path)]);
    }

    /// Parse `collect`'s files off the main thread and queue them, keeping
    /// `loading()` true until `Asset` drains the result. A parse panic (bad
    /// file) is contained here instead of taking down the app.
    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_load(collect: impl FnOnce() -> Vec<File> + Send + 'static) {
        Self::set_loading(true);
        std::thread::spawn(move || {
            let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Self::load_files(collect())
            }));
            if parsed.is_err() {
                log::error!("file load failed");
                Self::set_loading(false);
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn_load(collect: impl FnOnce() -> Vec<File> + 'static) {
        Self::set_loading(true);
        wasm_bindgen_futures::spawn_local(async move { Self::load_files(collect()) });
    }

    fn load_files(files: Vec<File>) {
        let mut lines: Vec<LineFile> = Vec::new();
        let mut track_scalars: Vec<TrackScalarFile> = Vec::new();
        let mut volumes: Vec<VolumeFile> = Vec::new();
        let mut hdris: Vec<HdriFile> = Vec::new();

        for file in files {
            if file.name().ends_with(".tck") {
                lines.push(LineFile::from_tck(file));
            } else if file.name().ends_with(".obj") {
                lines.push(LineFile::from_obj(file));
            } else if file.name().ends_with(".tsf") {
                track_scalars.push(TrackScalarFile::from_tsf(file));
            } else if file.name().ends_with(".nii") {
                volumes.push(VolumeFile::from_nifti(&file));
            } else if file.name().ends_with(".nii.gz") {
                volumes.push(VolumeFile::from_comressed_nifti(&file));
            } else if file.name().ends_with(".exr") {
                hdris.push(HdriFile::from_exr(&file));
            } else {
                warn!(
                    "Cannot open `{}`. Supported file types are [.tck .obj .nii.gz]",
                    file.name()
                )
            }
        }

        let queued = !lines.is_empty()
            || !track_scalars.is_empty()
            || !volumes.is_empty()
            || !hdris.is_empty();

        if let Ok(stage) = QUEUE.lock().as_mut() {
            stage.lines.extend(lines);
            stage.track_scalars.extend(track_scalars);
            stage.volumes.extend(volumes);
            stage.hdris.extend(hdris);
        }

        // Nothing landed in the queue (all files unsupported), so `Asset` will
        // never call `finish_loading` - clear the spinner here instead.
        if !queued {
            Self::set_loading(false);
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

    /// Browsers have no notion of a save-file dialog with filesystem access,
    /// so there's no path to pick - just queue the default filename and let
    /// `Gpu::save`'s web delivery path trigger a browser download.
    #[cfg(target_arch = "wasm32")]
    pub fn save() {
        Self::publish_save_path(PathBuf::from("screenshot.png"));
    }

    pub fn on_lines(callback: impl FnOnce(Vec<LineFile>)) {
        let mut data = QUEUE.lock().unwrap();

        if !data.lines.is_empty() {
            callback(data.lines.drain(..).collect());
        }
    }

    pub fn on_track_scalars(callback: impl FnOnce(Vec<TrackScalarFile>)) {
        let mut data = QUEUE.lock().unwrap();

        if !data.track_scalars.is_empty() {
            callback(data.track_scalars.drain(..).collect());
        }
    }

    pub fn on_volumes(callback: impl FnOnce(Vec<VolumeFile>)) {
        let mut data = QUEUE.lock().unwrap();

        if !data.volumes.is_empty() {
            callback(data.volumes.drain(..).collect());
        }
    }

    pub fn on_hdris(callback: impl FnOnce(Vec<HdriFile>)) {
        let mut data = QUEUE.lock().unwrap();

        if !data.hdris.is_empty() {
            callback(data.hdris.drain(..).collect());
        }
    }

    pub fn on_save(callback: impl FnOnce(PathBuf)) {
        let mut data = QUEUE.lock().unwrap();

        if let Some(save) = data.save.take() {
            callback(save);
        }
    }

    fn publish_save_path(path: PathBuf) {
        QUEUE.lock().unwrap().save = Some(path);
    }

    pub fn load_volume_fractions() {
        todo!()
    }
}

static QUEUE: LazyLock<Mutex<FileStage>> = LazyLock::new(|| Mutex::new(FileStage::default()));

static LOADING: AtomicBool = AtomicBool::new(false);

#[allow(clippy::type_complexity)]
static WAKER: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);
