use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct NativeFile {
    path: PathBuf,
}

impl From<PathBuf> for NativeFile {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl egui::DroppedFile for NativeFile {
    fn path(&self) -> &Path {
        &self.path
    }

    // Upstream bug: `egui::DroppedFile` requires `bytes` on native and `bytes_async` on
    // wasm32, but `egui-winit` 0.36.1 only ever implements `bytes`, so it fails to compile
    // for wasm32 (https://github.com/emilk/egui/issues/7052). `winit`'s web backend never
    // emits `WindowEvent::DroppedFile`, so this impl is unreachable there in practice - it
    // only needs to satisfy the trait.
    #[cfg(not(target_arch = "wasm32"))]
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.path).map_err(|err| err.to_string())
    }

    #[cfg(target_arch = "wasm32")]
    fn bytes_async(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + '_>> {
        Box::pin(std::future::ready(Err(
            "reading a dropped file by path is not supported on the web".to_owned(),
        )))
    }
}
