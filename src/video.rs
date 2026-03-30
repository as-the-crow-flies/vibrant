use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct VideoEncoder {
    child: Option<Child>,
    width: u32,
    height: u32,
}

impl VideoEncoder {
    pub fn new(output: &Path, width: u32, height: u32, fps: u32) -> io::Result<Self> {
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
                &format!("{}x{}", width, height),
                "-framerate",
                &fps.to_string(),
                "-i",
                "-",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
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

    pub fn write_frame(&mut self, data: &[u8]) -> io::Result<()> {
        let expected = (self.width * self.height * 4) as usize;
        if data.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "frame data size mismatch: got {} bytes, expected {}",
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

    pub fn finish(mut self) -> io::Result<()> {
        let mut child = self
            .child
            .take()
            .ok_or_else(|| io::Error::other("encoder already finished"))?;

        drop(child.stdin.take());
        let output = child.wait_with_output()?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(io::Error::other(format!(
                "ffmpeg exited with status {}: {stderr}",
                output.status
            )))
        }
    }
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            drop(child.stdin.take());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let output = dir.path().join("test.mp4");
        let mut encoder = VideoEncoder::new(&output, 64, 64, 30).expect("failed to start encoder");

        let result = encoder.write_frame(&vec![0; 100]);
        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expected size mismatch").kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn test_video_encoder_roundtrip() {
        if !ffmpeg_available() {
            eprintln!("Skipping test_video_encoder_roundtrip: ffmpeg not in PATH");
            return;
        }

        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let output = dir.path().join("roundtrip.mp4");
        let mut encoder = VideoEncoder::new(&output, 64, 64, 30).expect("failed to start encoder");
        let frame = vec![128u8; 64 * 64 * 4];

        for _ in 0..3 {
            encoder.write_frame(&frame).expect("failed to write frame");
        }

        encoder.finish().expect("failed to finalize video");

        assert!(output.exists(), "expected output video file to exist");
        assert!(
            std::fs::metadata(&output)
                .expect("failed to stat output video")
                .len()
                > 0,
            "expected output video file to be non-empty"
        );
    }
}
