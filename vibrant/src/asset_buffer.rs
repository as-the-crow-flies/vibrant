use std::{cell::RefCell, rc::Rc};

use tractogram::Tractogram;

use crate::{gpu::Gpu, loader};

pub mod tractogram;

pub struct AssetBuffer {
    tractogram: Rc<RefCell<Tractogram>>,
}

impl AssetBuffer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            tractogram: Self::new_tractogram(gpu, &loader::Tractogram::default()),
        }
    }

    pub fn new_tractogram(gpu: &Gpu, tractogram: &loader::Tractogram) -> Rc<RefCell<Tractogram>> {
        Rc::new(RefCell::new(Tractogram::new(gpu, tractogram)))
    }

    pub fn set_tractogram(&self, gpu: &Gpu, tractogram: &loader::Tractogram) {
        self.tractogram.replace(Tractogram::new(gpu, tractogram));
    }

    pub fn tractogram(&self) -> Rc<RefCell<Tractogram>> {
        Rc::clone(&self.tractogram)
    }
}
