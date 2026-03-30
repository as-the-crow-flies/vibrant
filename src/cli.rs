use std::path::PathBuf;

use clap::Parser;

/// VIBRANT — GPU-accelerated tractography visualization
#[derive(Parser, Debug)]
#[command(name = "vibrant", version, about)]
pub struct CliArgs {
    /// Input file(s) to load (.tck, .obj, .nii.gz)
    #[arg(short, long)]
    pub input: Vec<PathBuf>,

    /// Save a screenshot to this path and exit
    #[arg(long, conflicts_with = "video")]
    pub screenshot: Option<PathBuf>,

    /// Render a video to this path and exit
    #[arg(long, conflicts_with = "screenshot")]
    pub video: Option<PathBuf>,

    /// Video frame rate (default: 30)
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..))]
    pub fps: u32,

    /// Video duration in seconds (default: 10)
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..))]
    pub duration: u32,

    /// Enable auto-rotation of the camera
    #[arg(long)]
    pub auto_rotate: bool,

    /// Auto-rotation speed in degrees per second (default: 10.0)
    #[arg(long, default_value_t = 10.0)]
    pub rotate_speed: f32,

    /// Disable visual effects (bloom, etc.) in screenshots and video
    #[arg(long)]
    pub disable_visual_effects: bool,

    #[arg(long)]
    pub zoom: Option<f32>,
}

impl CliArgs {
    pub fn into_config(self) -> crate::app::AppConfig {
        use crate::app::{AppConfig, AppMode};

        let mode = if let Some(ref path) = self.screenshot {
            AppMode::Screenshot {
                output: path.clone(),
                zoom: self.zoom,
            }
        } else if let Some(ref path) = self.video {
            AppMode::Video {
                output: path.clone(),
                fps: self.fps,
                duration: self.duration,
                zoom: self.zoom,
            }
        } else {
            AppMode::Interactive
        };

        AppConfig {
            input: self.input,
            mode,
            auto_rotate: self.auto_rotate,
            rotate_speed: self.rotate_speed,
            disable_visual_effects: self.disable_visual_effects,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppMode;
    use clap::Parser;

    #[test]
    fn test_cli_default_interactive() {
        // No flags → Interactive mode
        let args = CliArgs::parse_from(["vibrant"]);
        let config = args.into_config();
        assert_eq!(config.mode, AppMode::Interactive);
        assert!(config.input.is_empty());
        assert!(!config.auto_rotate);
        assert_eq!(config.rotate_speed, 10.0);
    }

    #[test]
    fn test_cli_screenshot_mode() {
        let args = CliArgs::parse_from(["vibrant", "--screenshot", "out.png"]);
        let config = args.into_config();
        match &config.mode {
            AppMode::Screenshot { output, zoom } => {
                assert_eq!(output, &PathBuf::from("out.png"));
                assert_eq!(*zoom, None);
            }
            other => panic!("Expected Screenshot mode, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_accepts_zoom_argument_for_screenshot_mode() {
        let args = CliArgs::try_parse_from([
            "vibrant",
            "--input",
            "AF_left.tck",
            "--screenshot",
            "out.png",
            "--zoom",
            "1.5",
        ]);

        assert!(
            args.is_ok(),
            "expected --zoom <distance> baseline to parse successfully for screenshot mode"
        );

        let config = args.unwrap().into_config();
        match config.mode {
            AppMode::Screenshot { zoom, .. } => assert_eq!(zoom, Some(1.5)),
            other => panic!("Expected Screenshot mode, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_video_mode() {
        let args = CliArgs::parse_from([
            "vibrant",
            "--video",
            "out.mp4",
            "--fps",
            "60",
            "--duration",
            "5",
            "--zoom",
            "1.5",
        ]);

        let config = args.into_config();
        match config.mode {
            AppMode::Video {
                output,
                fps,
                duration,
                zoom,
            } => {
                assert_eq!(output, PathBuf::from("out.mp4"));
                assert_eq!(fps, 60);
                assert_eq!(duration, 5);
                assert_eq!(zoom, Some(1.5));
            }
            other => panic!("Expected Video mode, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_rejects_screenshot_and_video_together() {
        let args =
            CliArgs::try_parse_from(["vibrant", "--screenshot", "out.png", "--video", "out.mp4"]);

        assert!(args.is_err());
    }
}
