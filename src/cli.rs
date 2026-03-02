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

    /// Enable auto-rotation of the camera
    #[arg(long)]
    pub auto_rotate: bool,

    /// Auto-rotation speed in degrees per second (default: 10.0)
    #[arg(long, default_value_t = 10.0)]
    pub rotate_speed: f32,

    /// Enable post-processing effects (bloom) for screenshots and video
    #[arg(long)]
    pub render_effects: bool,
}

impl CliArgs {
    pub fn into_config(self) -> crate::app::AppConfig {
        use crate::app::{AppConfig, AppMode};

        let mode = if let Some(ref path) = self.screenshot {
            AppMode::Screenshot {
                output: path.clone(),
            }
        } else {
            AppMode::Interactive
        };

        AppConfig {
            input: self.input,
            mode,
            auto_rotate: self.auto_rotate,
            rotate_speed: self.rotate_speed,
            render_effects: self.render_effects,
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
            AppMode::Screenshot { output } => {
                assert_eq!(output, &PathBuf::from("out.png"));
            }
            other => panic!("Expected Screenshot mode, got {:?}", other),
        }
    }
}
