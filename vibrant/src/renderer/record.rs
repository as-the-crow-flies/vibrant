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
                "rgba",
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

    pub fn new_hdr(width: u32, height: u32, fps: u32, output_path: &str) -> Self {
        let mut process = Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "rawvideo",
                "-pixel_format", "rgba64le",
                "-video_size", &format!("{}x{}", width, height),
                "-framerate", &fps.to_string(),
                "-i", "pipe:0",
                "-c:v", "libx265",
                "-pix_fmt", "yuv420p10le",
                "-color_primaries", "bt709", 
                "-color_trc", "smpte2084",       
                "-colorspace", "bt709",       
                "-x265-params",
                "hdr-opt=1:repeat-headers=1:colorprim=bt709:transfer=smpte2084:colormatrix=bt709:master-display=G(15000,30000)B(7500,3000)R(32000,16500)WP(15635,16450)L(10000000,50):max-cll=1000,400",
                "-tag:v", "hvc1",
                "-crf", "18",
                output_path,
            ])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start FFmpeg");

        let stdin = process.stdin.take().unwrap();
        println!(
            "HDR recorder spawned: {}x{} @ {}fps -> {}",
            width, height, fps, output_path
        );

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
