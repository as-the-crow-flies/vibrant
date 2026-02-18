use std::io::{self, Write};
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
    ///
    /// Returns an error if the output directory cannot be created or ffmpeg
    /// cannot be spawned (e.g. not installed / not in PATH).
    pub fn new(output: &Path, width: u32, height: u32, fps: u32) -> io::Result<Self> {
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
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
            .spawn()?;

        Ok(Self {
            child,
            width,
            height,
        })
    }

    /// Write a single RGBA frame to ffmpeg's stdin.
    ///
    /// Returns an error if the frame data size is wrong or the write fails
    /// (e.g. ffmpeg has exited / pipe broken).
    pub fn write_frame(&mut self, data: &[u8]) -> io::Result<()> {
        let expected = (self.width * self.height * 4) as usize;
        if data.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Frame data size mismatch: got {} bytes, expected {}",
                    data.len(),
                    expected
                ),
            ));
        }

        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "ffmpeg stdin unavailable"))?;
        stdin.write_all(data)
    }

    /// Close stdin and wait for ffmpeg to finish encoding.
    ///
    /// Returns an error if ffmpeg exits with a non-zero status.
    pub fn finish(mut self) -> io::Result<()> {
        // Drop stdin to signal EOF
        drop(self.child.stdin.take());

        let output = self.child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg exited with status: {}. stderr:\n{}",
                    output.status, stderr
                ),
            ))
        } else {
            log::info!("ffmpeg finished successfully");
            Ok(())
        }
    }
}
