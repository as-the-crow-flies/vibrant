pub mod app;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
async fn start() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("Couldn't initialize console_log");

    app::run().await;
}
