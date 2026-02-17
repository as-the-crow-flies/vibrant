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
    #[arg(long)]
    pub screenshot: Option<PathBuf>,

    /// Render a video to this path and exit
    #[arg(long)]
    pub video: Option<PathBuf>,

    /// Video frame rate (default: 30)
    #[arg(long, default_value_t = 30)]
    pub fps: u32,

    /// Video duration in seconds (default: 10)
    #[arg(long, default_value_t = 10)]
    pub duration: u32,

    /// Enable auto-rotation of the camera
    #[arg(long)]
    pub auto_rotate: bool,

    /// Auto-rotation speed in degrees per second (default: 10.0)
    #[arg(long, default_value_t = 10.0)]
    pub rotate_speed: f32,
}

impl CliArgs {
    pub fn into_config(self) -> crate::app::AppConfig {
        use crate::app::{AppConfig, AppMode};

        let mode = if let Some(ref path) = self.screenshot {
            AppMode::Screenshot {
                output: path.clone(),
            }
        } else if let Some(ref path) = self.video {
            AppMode::Video {
                output: path.clone(),
                fps: self.fps,
                duration: self.duration,
            }
        } else {
            AppMode::Interactive
        };

        AppConfig {
            input: self.input,
            mode,
            auto_rotate: self.auto_rotate,
            rotate_speed: self.rotate_speed,
        }
    }
}
