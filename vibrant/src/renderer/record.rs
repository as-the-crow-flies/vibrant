use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};

pub struct Recorder {
    process: Child,
    stdin: ChildStdin,
}

impl Recorder {
    pub fn new(width: u32, height: u32, fps: u32, output_path: &str) -> Self {
        println!(
            "Spawning FFmpeg: {}x{} @ {}fps -> {}",
            width, height, fps, output_path
        );

        let mut process = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "bgra",
                "-video_size",
                &format!("{}x{}", width, height),
                "-framerate",
                &fps.to_string(), // input fps = fixed recording fps
                "-i",
                "pipe:0",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-crf",
                "18",
                "-r",
                &fps.to_string(), // output fps = same
                output_path,
            ])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start FFmpeg");

        let stdin = process.stdin.take().unwrap();

        println!("FFmpeg spawned successfully, PID: {:?}", process.id());

        Self { process, stdin }
    }

    pub fn write_frame(&mut self, pixels: &[u8]) {
        if let Err(e) = self.stdin.write_all(pixels) {
            eprintln!("Failed to write frame to FFmpeg: {}", e);
        }
    }

    pub fn finish(mut self) {
        println!("Finishing recording...");

        drop(self.stdin); // signals EOF to FFmpeg so it finalizes the file
        let output = self
            .process
            .wait_with_output()
            .expect("FFmpeg did not finish cleanly");

        if !output.stderr.is_empty() {
            println!(
                "FFmpeg output:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        println!("Recording saved.");
    }
}
