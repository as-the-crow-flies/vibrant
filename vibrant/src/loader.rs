pub mod tractogram;

pub use tractogram::*;

use std::sync::{LazyLock, Mutex};

#[derive(Default)]
pub struct AssetLoader {
    pub tractogram: Option<Tractogram>,
}

impl AssetLoader {
    pub fn publish_tractogram(tractogram: Tractogram) {
        QUEUE.lock().unwrap().tractogram = Some(tractogram);
    }

    pub fn on_tractogram(callback: impl FnOnce(Tractogram)) {
        let mut data = QUEUE.lock().unwrap();

        if let Some(tractogram) = data.tractogram.take() {
            callback(tractogram);
        }
    }
}

static QUEUE: LazyLock<Mutex<AssetLoader>> = LazyLock::new(|| Mutex::new(AssetLoader::default()));
