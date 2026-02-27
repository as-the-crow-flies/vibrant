pub mod bounds;
pub mod line;
pub mod volume;

pub use line::*;
pub use volume::*;

use std::{
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use log::warn;

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

#[derive(Default)]
pub struct FileStage {
    pub lines: Vec<LineFile>,
    pub volumes: Vec<VolumeFile>,
    pub save: Option<PathBuf>,
}

impl FileStage {
    #[cfg(target_arch = "wasm32")]
    pub fn load() {
        wasm_bindgen_futures::spawn_local(async move {
            let mut files: Vec<File> = Vec::new();

            if let Some(handles) = rfd::AsyncFileDialog::new().pick_files().await {
                for handle in handles {
                    files.push(File {
                        name: handle.file_name(),
                        data: handle.read().await,
                    });
                }
            }

            Self::load_files(files);
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() {
        use std::fs;

        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            Self::load_files(
                paths
                    .into_iter()
                    .map(|path| File {
                        name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                        data: fs::read(&path).expect(&format!(
                            "should be able to read path: `{:?}`",
                            path.to_str()
                        )),
                    })
                    .collect(),
            );
        }
    }

    fn load_files(files: Vec<File>) {
        let mut lines: Vec<LineFile> = Vec::new();
        let mut volumes: Vec<VolumeFile> = Vec::new();

        for file in files {
            if file.name().ends_with(".tck") {
                lines.push(LineFile::from_tck(file));
            } else if file.name().ends_with(".obj") {
                lines.push(LineFile::from_obj(file));
            } else if file.name().ends_with(".nii.gz") {
                volumes.push(VolumeFile::from_nifti(&file));
            } else {
                warn!(
                    "Cannot open `{}`. Supported file types are [.tck .obj .nii.gz]",
                    file.name()
                )
            }
        }

        if !lines.is_empty() {
            QUEUE.lock().unwrap().lines.extend(lines);
        }

        if !volumes.is_empty() {
            QUEUE.lock().unwrap().volumes.extend(volumes);
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

    pub fn on_lines(callback: impl FnOnce(Vec<LineFile>)) {
        let mut data = QUEUE.lock().unwrap();

        if !data.lines.is_empty() {
            callback(data.lines.drain(..).collect());
        }
    }

    pub fn on_volumes(callback: impl FnOnce(Vec<VolumeFile>)) {
        let mut data = QUEUE.lock().unwrap();

        if !data.volumes.is_empty() {
            callback(data.volumes.drain(..).collect());
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

    fn publish_save_path(path: PathBuf) {
        QUEUE.lock().unwrap().save = Some(path);
    }

    pub fn load_volume_fractions() {
        todo!()
    }
}

static QUEUE: LazyLock<Mutex<FileStage>> = LazyLock::new(|| Mutex::new(FileStage::default()));
