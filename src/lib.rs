pub mod app;
pub mod asset;
pub mod common;
pub mod controller;
pub mod file;
pub mod gpu;
pub mod renderer;
pub mod surface;
pub mod util;

pub use glam::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
async fn start() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("Couldn't initialize console_log");

    app::run().await;
}
