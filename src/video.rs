use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Manages an ffmpeg subprocess that receives raw RGBA frames via stdin
/// and encodes them into an MP4 (H.264) file.
pub struct VideoEncoder {
    child: Option<Child>,
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
            child: Some(child),
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

        let child = self
            .child
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "encoder already finished"))?;
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "ffmpeg stdin unavailable"))?;
        stdin.write_all(data)
    }

    /// Close stdin and wait for ffmpeg to finish encoding.
    ///
    /// Returns an error if ffmpeg exits with a non-zero status.
    pub fn finish(mut self) -> io::Result<()> {
        let mut child = self
            .child
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "encoder already finished"))?;

        // Drop stdin to signal EOF
        drop(child.stdin.take());

        let output = child.wait_with_output()?;
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

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            // Close stdin so ffmpeg sees EOF, then kill if still running and
            // reap the child to prevent zombie processes.
            drop(child.stdin.take());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn ffmpeg_available() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    #[test]
    fn test_write_frame_size_mismatch() {
        if !ffmpeg_available() {
            eprintln!("Skipping test_write_frame_size_mismatch: ffmpeg not in PATH");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("test.mp4");
        let mut encoder = VideoEncoder::new(&output, 64, 64, 30).unwrap();

        // Correct size = 64*64*4 = 16384. Send a wrong size.
        let wrong_data = vec![0u8; 100];
        let result = encoder.write_frame(&wrong_data);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

        // Clean up: Drop impl kills and reaps the child process
        drop(encoder);
    }

    #[test]
    fn test_video_encoder_roundtrip() {
        if !ffmpeg_available() {
            eprintln!("Skipping test_video_encoder_roundtrip: ffmpeg not in PATH");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("roundtrip.mp4");
        let w = 64u32;
        let h = 64u32;
        let mut encoder = VideoEncoder::new(&output, w, h, 30).unwrap();

        // Write 3 dummy frames (solid color)
        let frame = vec![128u8; (w * h * 4) as usize];
        for _ in 0..3 {
            encoder.write_frame(&frame).unwrap();
        }

        encoder.finish().unwrap();

        // Verify the output file exists and is non-empty
        assert!(output.exists(), "Output file should exist");
        let metadata = std::fs::metadata(&output).unwrap();
        assert!(metadata.len() > 0, "Output file should be non-empty");
    }
}
