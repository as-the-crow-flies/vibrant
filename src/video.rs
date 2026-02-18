use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Manages an ffmpeg subprocess that receives raw RGBA frames via stdin
/// and encodes them into an MP4 (H.264) file.
pub struct VideoEncoder {
    child: Child,
    width: u32,
    height: u32,
}

impl VideoEncoder {
    /// Spawn an ffmpeg process ready to receive raw RGBA frames.
    pub fn new(output: &Path, width: u32, height: u32, fps: u32) -> Self {
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).unwrap();
            }
        }

        let child = Command::new("ffmpeg")
            .args([
                "-y", // overwrite output
                "-f",
                "rawvideo", // input format
                "-pixel_format",
                "rgba", // pixel format
                "-video_size",
                &format!("{}x{}", width, height),
                "-framerate",
                &fps.to_string(),
                "-i",
                "-", // read from stdin
                "-c:v",
                "libx264", // H.264 codec
                "-pix_fmt",
                "yuv420p", // compatible pixel format
                "-preset",
                "medium", // encoding speed/quality tradeoff
                "-crf",
                "18", // quality (lower = better, 18 is visually lossless)
                "-movflags",
                "+faststart", // web-friendly MP4
            ])
            .arg(output.as_os_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start ffmpeg. Is ffmpeg installed and in PATH?");

        Self {
            child,
            width,
            height,
        }
    }

    /// Write a single RGBA frame to ffmpeg's stdin.
    pub fn write_frame(&mut self, data: &[u8]) {
        let expected = (self.width * self.height * 4) as usize;
        assert_eq!(
            data.len(),
            expected,
            "Frame data size mismatch: got {} bytes, expected {}",
            data.len(),
            expected
        );

        let stdin = self.child.stdin.as_mut().expect("ffmpeg stdin");
        stdin
            .write_all(data)
            .expect("Failed to write frame to ffmpeg");
    }

    /// Close stdin and wait for ffmpeg to finish encoding.
    pub fn finish(mut self) {
        // Drop stdin to signal EOF
        drop(self.child.stdin.take());

        let output = self
            .child
            .wait_with_output()
            .expect("Failed to wait for ffmpeg");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!(
                "ffmpeg exited with status: {}. stderr:\n{}",
                output.status,
                stderr
            );
        } else {
            log::info!("ffmpeg finished successfully");
        }
    }
}
