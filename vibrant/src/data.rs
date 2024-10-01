use std::sync::{LazyLock, Mutex};

use crate::loader::Tractogram;

#[derive(Default)]
pub struct Data {
    pub tractogram: Option<Tractogram>,
}

impl Data {
    pub fn set_tractogram(tractogram: Tractogram) {
        DATA.lock().unwrap().tractogram = Some(tractogram);
    }

    pub fn pop_tractogram(callback: impl FnOnce(&Tractogram)) {
        let mut data = DATA.lock().unwrap();

        if let Some(tractogram) = data.tractogram.as_ref() {
            callback(tractogram);
        }

        data.tractogram = None;
    }
}

static DATA: LazyLock<Mutex<Data>> = LazyLock::new(|| Mutex::new(Data::default()));
